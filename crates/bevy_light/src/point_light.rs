use bevy_asset::Handle;
use bevy_camera::{
    primitives::{CubeMapFace, CubemapFrusta, CubemapLayout, Frustum, CUBE_MAP_FACES},
    visibility::{self, CubemapVisibleEntities, Visibility, VisibilityClass},
};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::Mat4;
use bevy_reflect::prelude::*;
use bevy_transform::components::{GlobalTransform, Transform};

use crate::{
    cluster::{ClusterVisibilityClass, GlobalVisibleClusterableObjects},
    light_consts,
};

/// A light that emits light in all directions from a central point.
///
/// Real-world values for `intensity` (luminous power in lumens) based on the electrical power
/// consumption of the type of real-world light are:
///
/// | Luminous Power (lumen) (i.e. the intensity member) | Incandescent non-halogen (Watts) | Incandescent halogen (Watts) | Compact fluorescent (Watts) | LED (Watts) |
/// |------|-----|----|--------|-------|
/// | 200  | 25  |    | 3-5    | 3     |
/// | 450  | 40  | 29 | 9-11   | 5-8   |
/// | 800  | 60  |    | 13-15  | 8-12  |
/// | 1100 | 75  | 53 | 18-20  | 10-16 |
/// | 1600 | 100 | 72 | 24-28  | 14-17 |
/// | 2400 | 150 |    | 30-52  | 24-30 |
/// | 3100 | 200 |    | 49-75  | 32    |
/// | 4000 | 300 |    | 75-100 | 40.5  |
///
/// Source: [Wikipedia](https://en.wikipedia.org/wiki/Lumen_(unit)#Lighting)
///
/// ## Shadows
///
/// To enable shadows, set the `shadows_enabled` property to `true`.
///
/// To control the resolution of the shadow maps, use the [`PointLightShadowMap`] resource.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(
    CubemapFrusta,
    CubemapVisibleEntities,
    Transform,
    Visibility,
    VisibilityClass
)]
#[component(on_add = visibility::add_visibility_class::<ClusterVisibilityClass>)]
pub struct PointLight {
    /// The color of this light source.
    pub color: Color,

    /// Luminous power in lumens, representing the amount of light emitted by this source in all directions.
    pub intensity: f32,

    /// Cut-off for the light's area-of-effect. Fragments outside this range will not be affected by
    /// this light at all, so it's important to tune this together with `intensity` to prevent hard
    /// lighting cut-offs.
    pub range: f32,

    /// Simulates a light source coming from a spherical volume with the given
    /// radius.
    ///
    /// This affects the size of specular highlights created by this light, as
    /// well as the soft shadow penumbra size. Because of this, large values may
    /// not produce the intended result -- for example, light radius does not
    /// affect shadow softness or diffuse lighting.
    pub radius: f32,

    /// Whether this light casts shadows.
    pub shadows_enabled: bool,

    /// Whether soft shadows are enabled.
    ///
    /// Soft shadows, also known as *percentage-closer soft shadows* or PCSS,
    /// cause shadows to become blurrier (i.e. their penumbra increases in
    /// radius) as they extend away from objects. The blurriness of the shadow
    /// depends on the [`PointLight::radius`] of the light; larger lights result
    /// in larger penumbras and therefore blurrier shadows.
    ///
    /// Currently, soft shadows are rather noisy if not using the temporal mode.
    /// If you enable soft shadows, consider choosing
    /// [`ShadowFilteringMethod::Temporal`] and enabling temporal antialiasing
    /// (TAA) to smooth the noise out over time.
    ///
    /// Note that soft shadows are significantly more expensive to render than
    /// hard shadows.
    ///
    /// [`ShadowFilteringMethod::Temporal`]: crate::ShadowFilteringMethod::Temporal
    #[cfg(feature = "experimental_pbr_pcss")]
    pub soft_shadows_enabled: bool,

    /// Whether this point light contributes diffuse lighting to meshes with
    /// lightmaps.
    ///
    /// Set this to false if your lightmap baking tool bakes the direct diffuse
    /// light from this point light into the lightmaps in order to avoid
    /// counting the radiance from this light twice. Note that the specular
    /// portion of the light is always considered, because Bevy currently has no
    /// means to bake specular light.
    ///
    /// By default, this is set to true.
    pub affects_lightmapped_mesh_diffuse: bool,

    /// A bias used when sampling shadow maps to avoid "shadow-acne", or false shadow occlusions
    /// that happen as a result of shadow-map fragments not mapping 1:1 to screen-space fragments.
    /// Too high of a depth bias can lead to shadows detaching from their casters, or
    /// "peter-panning". This bias can be tuned together with `shadow_normal_bias` to correct shadow
    /// artifacts for a given scene.
    pub shadow_depth_bias: f32,

    /// A bias applied along the direction of the fragment's surface normal. It is scaled to the
    /// shadow map's texel size so that it can be small close to the camera and gets larger further
    /// away.
    pub shadow_normal_bias: f32,

    /// The distance from the light to near Z plane in the shadow map.
    ///
    /// Objects closer than this distance to the light won't cast shadows.
    /// Setting this higher increases the shadow map's precision.
    ///
    /// This only has an effect if shadows are enabled.
    pub shadow_map_near_z: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        PointLight {
            color: Color::WHITE,
            intensity: light_consts::lumens::VERY_LARGE_CINEMA_LIGHT,
            range: 20.0,
            radius: 0.0,
            shadows_enabled: false,
            affects_lightmapped_mesh_diffuse: true,
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
            shadow_map_near_z: Self::DEFAULT_SHADOW_MAP_NEAR_Z,
            #[cfg(feature = "experimental_pbr_pcss")]
            soft_shadows_enabled: false,
        }
    }
}

impl PointLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.08;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 0.6;
    pub const DEFAULT_SHADOW_MAP_NEAR_Z: f32 = 0.1;
}

/// Add to a [`PointLight`] to add a light texture effect.
/// A texture mask is applied to the light source to modulate its intensity,  
/// simulating patterns like window shadows, gobo/cookie effects, or soft falloffs.
#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Component, Debug)]
#[require(PointLight)]
pub struct PointLightTexture {
    /// The texture image. Only the R channel is read.
    pub image: Handle<Image>,
    /// The cubemap layout. The image should be a packed cubemap in one of the formats described by the [`CubemapLayout`] enum.
    pub cubemap_layout: CubemapLayout,
}

/// Controls the resolution of [`PointLight`] shadow maps.
///
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_light::PointLightShadowMap;
/// App::new()
///     .insert_resource(PointLightShadowMap { size: 2048 });
/// ```
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource, Debug, Default, Clone)]
pub struct PointLightShadowMap {
    /// The width and height of each of the 6 faces of the cubemap.
    ///
    /// Defaults to `1024`.
    pub size: usize,
}

impl Default for PointLightShadowMap {
    fn default() -> Self {
        Self { size: 1024 }
    }
}

// NOTE: Run this after assign_lights_to_clusters!
pub fn update_point_light_frusta(
    global_lights: Res<GlobalVisibleClusterableObjects>,
    mut views: Query<(Entity, &GlobalTransform, &PointLight, &mut CubemapFrusta)>,
    changed_lights: Query<
        Entity,
        (
            With<PointLight>,
            Or<(Changed<GlobalTransform>, Changed<PointLight>)>,
        ),
    >,
) {
    let view_rotations = CUBE_MAP_FACES
        .iter()
        .map(|CubeMapFace { target, up }| Transform::IDENTITY.looking_at(*target, *up))
        .collect::<Vec<_>>();

    for (entity, transform, point_light, mut cubemap_frusta) in &mut views {
        // If this light hasn't changed, and neither has the set of global_lights,
        // then we can skip this calculation.
        if !global_lights.is_changed() && !changed_lights.contains(entity) {
            continue;
        }

        // The frusta are used for culling meshes to the light for shadow mapping
        // so if shadow mapping is disabled for this light, then the frusta are
        // not needed.
        // Also, if the light is not relevant for any cluster, it will not be in the
        // global lights set and so there is no need to update its frusta.
        if !point_light.shadows_enabled || !global_lights.entities.contains(&entity) {
            continue;
        }

        let clip_from_view = Mat4::perspective_infinite_reverse_rh(
            core::f32::consts::FRAC_PI_2,
            1.0,
            point_light.shadow_map_near_z,
        );

        // ignore scale because we don't want to effectively scale light radius and range
        // by applying those as a view transform to shadow map rendering of objects
        // and ignore rotation because we want the shadow map projections to align with the axes
        let view_translation = Transform::from_translation(transform.translation());
        let view_backward = transform.back();

        for (view_rotation, frustum) in view_rotations.iter().zip(cubemap_frusta.iter_mut()) {
            let world_from_view = view_translation * *view_rotation;
            let clip_from_world = clip_from_view * world_from_view.compute_affine().inverse();

            *frustum = Frustum::from_clip_from_world_custom_far(
                &clip_from_world,
                &transform.translation(),
                &view_backward,
                point_light.range,
            );
        }
    }
}
