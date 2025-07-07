pub use bevy_camera::primitives::{face_index_to_name, CubeMapFace, CUBE_MAP_FACES};
use bevy_camera::{Camera, Projection};
use bevy_ecs::{entity::EntityHashMap, prelude::*};
use bevy_math::{ops, Mat4, Vec3A, Vec4};
use bevy_reflect::prelude::*;
use bevy_transform::components::GlobalTransform;

use crate::{DirectionalLight, DirectionalLightShadowMap};

/// Controls how cascaded shadow mapping works.
/// Prefer using [`CascadeShadowConfigBuilder`] to construct an instance.
///
/// ```
/// # use bevy_light::CascadeShadowConfig;
/// # use bevy_light::CascadeShadowConfigBuilder;
/// # use bevy_utils::default;
/// #
/// let config: CascadeShadowConfig = CascadeShadowConfigBuilder {
///   maximum_distance: 100.0,
///   ..default()
/// }.into();
/// ```
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct CascadeShadowConfig {
    /// The (positive) distance to the far boundary of each cascade.
    pub bounds: Vec<f32>,
    /// The proportion of overlap each cascade has with the previous cascade.
    pub overlap_proportion: f32,
    /// The (positive) distance to the near boundary of the first cascade.
    pub minimum_distance: f32,
}

impl Default for CascadeShadowConfig {
    fn default() -> Self {
        CascadeShadowConfigBuilder::default().into()
    }
}

fn calculate_cascade_bounds(
    num_cascades: usize,
    nearest_bound: f32,
    shadow_maximum_distance: f32,
) -> Vec<f32> {
    if num_cascades == 1 {
        return vec![shadow_maximum_distance];
    }
    let base = ops::powf(
        shadow_maximum_distance / nearest_bound,
        1.0 / (num_cascades - 1) as f32,
    );
    (0..num_cascades)
        .map(|i| nearest_bound * ops::powf(base, i as f32))
        .collect()
}

/// Builder for [`CascadeShadowConfig`].
pub struct CascadeShadowConfigBuilder {
    /// The number of shadow cascades.
    /// More cascades increases shadow quality by mitigating perspective aliasing - a phenomenon where areas
    /// nearer the camera are covered by fewer shadow map texels than areas further from the camera, causing
    /// blocky looking shadows.
    ///
    /// This does come at the cost increased rendering overhead, however this overhead is still less
    /// than if you were to use fewer cascades and much larger shadow map textures to achieve the
    /// same quality level.
    ///
    /// In case rendered geometry covers a relatively narrow and static depth relative to camera, it may
    /// make more sense to use fewer cascades and a higher resolution shadow map texture as perspective aliasing
    /// is not as much an issue. Be sure to adjust `minimum_distance` and `maximum_distance` appropriately.
    pub num_cascades: usize,
    /// The minimum shadow distance, which can help improve the texel resolution of the first cascade.
    /// Areas nearer to the camera than this will likely receive no shadows.
    ///
    /// NOTE: Due to implementation details, this usually does not impact shadow quality as much as
    /// `first_cascade_far_bound` and `maximum_distance`. At many view frustum field-of-views, the
    /// texel resolution of the first cascade is dominated by the width / height of the view frustum plane
    /// at `first_cascade_far_bound` rather than the depth of the frustum from `minimum_distance` to
    /// `first_cascade_far_bound`.
    pub minimum_distance: f32,
    /// The maximum shadow distance.
    /// Areas further from the camera than this will likely receive no shadows.
    pub maximum_distance: f32,
    /// Sets the far bound of the first cascade, relative to the view origin.
    /// In-between cascades will be exponentially spaced relative to the maximum shadow distance.
    /// NOTE: This is ignored if there is only one cascade, the maximum distance takes precedence.
    pub first_cascade_far_bound: f32,
    /// Sets the overlap proportion between cascades.
    /// The overlap is used to make the transition from one cascade's shadow map to the next
    /// less abrupt by blending between both shadow maps.
    pub overlap_proportion: f32,
}

impl CascadeShadowConfigBuilder {
    /// Returns the cascade config as specified by this builder.
    pub fn build(&self) -> CascadeShadowConfig {
        assert!(
            self.num_cascades > 0,
            "num_cascades must be positive, but was {}",
            self.num_cascades
        );
        assert!(
            self.minimum_distance >= 0.0,
            "maximum_distance must be non-negative, but was {}",
            self.minimum_distance
        );
        assert!(
            self.num_cascades == 1 || self.minimum_distance < self.first_cascade_far_bound,
            "minimum_distance must be less than first_cascade_far_bound, but was {}",
            self.minimum_distance
        );
        assert!(
            self.maximum_distance > self.minimum_distance,
            "maximum_distance must be greater than minimum_distance, but was {}",
            self.maximum_distance
        );
        assert!(
            (0.0..1.0).contains(&self.overlap_proportion),
            "overlap_proportion must be in [0.0, 1.0) but was {}",
            self.overlap_proportion
        );
        CascadeShadowConfig {
            bounds: calculate_cascade_bounds(
                self.num_cascades,
                self.first_cascade_far_bound,
                self.maximum_distance,
            ),
            overlap_proportion: self.overlap_proportion,
            minimum_distance: self.minimum_distance,
        }
    }
}

impl Default for CascadeShadowConfigBuilder {
    fn default() -> Self {
        // The defaults are chosen to be similar to be Unity, Unreal, and Godot.
        // Unity: first cascade far bound = 10.05, maximum distance = 150.0
        // Unreal Engine 5: maximum distance = 200.0
        // Godot: first cascade far bound = 10.0, maximum distance = 100.0
        Self {
            // Currently only support one cascade in WebGL 2.
            num_cascades: if cfg!(all(
                feature = "webgl",
                target_arch = "wasm32",
                not(feature = "webgpu")
            )) {
                1
            } else {
                4
            },
            minimum_distance: 0.1,
            maximum_distance: 150.0,
            first_cascade_far_bound: 10.0,
            overlap_proportion: 0.2,
        }
    }
}

impl From<CascadeShadowConfigBuilder> for CascadeShadowConfig {
    fn from(builder: CascadeShadowConfigBuilder) -> Self {
        builder.build()
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
pub struct Cascades {
    /// Map from a view to the configuration of each of its [`Cascade`]s.
    pub cascades: EntityHashMap<Vec<Cascade>>,
}

#[derive(Clone, Debug, Default, Reflect)]
#[reflect(Clone, Default)]
pub struct Cascade {
    /// The transform of the light, i.e. the view to world matrix.
    pub world_from_cascade: Mat4,
    /// The orthographic projection for this cascade.
    pub clip_from_cascade: Mat4,
    /// The view-projection matrix for this cascade, converting world space into light clip space.
    /// Importantly, this is derived and stored separately from `view_transform` and `projection` to
    /// ensure shadow stability.
    pub clip_from_world: Mat4,
    /// Size of each shadow map texel in world units.
    pub texel_size: f32,
}

pub fn clear_directional_light_cascades(mut lights: Query<(&DirectionalLight, &mut Cascades)>) {
    for (directional_light, mut cascades) in lights.iter_mut() {
        if !directional_light.shadows_enabled {
            continue;
        }
        cascades.cascades.clear();
    }
}

pub fn build_directional_light_cascades(
    directional_light_shadow_map: Res<DirectionalLightShadowMap>,
    views: Query<(Entity, &GlobalTransform, &Projection, &Camera)>,
    mut lights: Query<(
        &GlobalTransform,
        &DirectionalLight,
        &CascadeShadowConfig,
        &mut Cascades,
    )>,
) {
    let views = views
        .iter()
        .filter_map(|(entity, transform, projection, camera)| {
            if camera.is_active {
                Some((entity, projection, transform.to_matrix()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    for (transform, directional_light, cascades_config, mut cascades) in &mut lights {
        if !directional_light.shadows_enabled {
            continue;
        }

        // It is very important to the numerical and thus visual stability of shadows that
        // light_to_world has orthogonal upper-left 3x3 and zero translation.
        // Even though only the direction (i.e. rotation) of the light matters, we don't constrain
        // users to not change any other aspects of the transform - there's no guarantee
        // `transform.to_matrix()` will give us a matrix with our desired properties.
        // Instead, we directly create a good matrix from just the rotation.
        let world_from_light = Mat4::from_quat(transform.compute_transform().rotation);
        let light_to_world_inverse = world_from_light.inverse();

        for (view_entity, projection, view_to_world) in views.iter().copied() {
            let camera_to_light_view = light_to_world_inverse * view_to_world;
            let view_cascades = cascades_config
                .bounds
                .iter()
                .enumerate()
                .map(|(idx, far_bound)| {
                    // Negate bounds as -z is camera forward direction.
                    let z_near = if idx > 0 {
                        (1.0 - cascades_config.overlap_proportion)
                            * -cascades_config.bounds[idx - 1]
                    } else {
                        -cascades_config.minimum_distance
                    };
                    let z_far = -far_bound;

                    let corners = projection.get_frustum_corners(z_near, z_far);

                    calculate_cascade(
                        corners,
                        directional_light_shadow_map.size as f32,
                        world_from_light,
                        camera_to_light_view,
                    )
                })
                .collect();
            cascades.cascades.insert(view_entity, view_cascades);
        }
    }
}

/// Returns a [`Cascade`] for the frustum defined by `frustum_corners`.
///
/// The corner vertices should be specified in the following order:
/// first the bottom right, top right, top left, bottom left for the near plane, then similar for the far plane.
fn calculate_cascade(
    frustum_corners: [Vec3A; 8],
    cascade_texture_size: f32,
    world_from_light: Mat4,
    light_from_camera: Mat4,
) -> Cascade {
    let mut min = Vec3A::splat(f32::MAX);
    let mut max = Vec3A::splat(f32::MIN);
    for corner_camera_view in frustum_corners {
        let corner_light_view = light_from_camera.transform_point3a(corner_camera_view);
        min = min.min(corner_light_view);
        max = max.max(corner_light_view);
    }

    // NOTE: Use the larger of the frustum slice far plane diagonal and body diagonal lengths as this
    //       will be the maximum possible projection size. Use the ceiling to get an integer which is
    //       very important for floating point stability later. It is also important that these are
    //       calculated using the original camera space corner positions for floating point precision
    //       as even though the lengths using corner_light_view above should be the same, precision can
    //       introduce small but significant differences.
    // NOTE: The size remains the same unless the view frustum or cascade configuration is modified.
    let cascade_diameter = (frustum_corners[0] - frustum_corners[6])
        .length()
        .max((frustum_corners[4] - frustum_corners[6]).length())
        .ceil();

    // NOTE: If we ensure that cascade_texture_size is a power of 2, then as we made cascade_diameter an
    //       integer, cascade_texel_size is then an integer multiple of a power of 2 and can be
    //       exactly represented in a floating point value.
    let cascade_texel_size = cascade_diameter / cascade_texture_size;
    // NOTE: For shadow stability it is very important that the near_plane_center is at integer
    //       multiples of the texel size to be exactly representable in a floating point value.
    let near_plane_center = Vec3A::new(
        (0.5 * (min.x + max.x) / cascade_texel_size).floor() * cascade_texel_size,
        (0.5 * (min.y + max.y) / cascade_texel_size).floor() * cascade_texel_size,
        // NOTE: max.z is the near plane for right-handed y-up
        max.z,
    );

    // It is critical for `world_to_cascade` to be stable. So rather than forming `cascade_to_world`
    // and inverting it, which risks instability due to numerical precision, we directly form
    // `world_to_cascade` as the reference material suggests.
    let light_to_world_transpose = world_from_light.transpose();
    let cascade_from_world = Mat4::from_cols(
        light_to_world_transpose.x_axis,
        light_to_world_transpose.y_axis,
        light_to_world_transpose.z_axis,
        (-near_plane_center).extend(1.0),
    );

    // Right-handed orthographic projection, centered at `near_plane_center`.
    // NOTE: This is different from the reference material, as we use reverse Z.
    let r = (max.z - min.z).recip();
    let clip_from_cascade = Mat4::from_cols(
        Vec4::new(2.0 / cascade_diameter, 0.0, 0.0, 0.0),
        Vec4::new(0.0, 2.0 / cascade_diameter, 0.0, 0.0),
        Vec4::new(0.0, 0.0, r, 0.0),
        Vec4::new(0.0, 0.0, 1.0, 1.0),
    );

    let clip_from_world = clip_from_cascade * cascade_from_world;
    Cascade {
        world_from_cascade: cascade_from_world.inverse(),
        clip_from_cascade,
        clip_from_world,
        texel_size: cascade_texel_size,
    }
}
