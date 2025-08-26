use bevy_asset::Handle;
use bevy_camera::{
    primitives::Frustum,
    visibility::{self, Visibility, VisibilityClass, VisibleMeshEntities},
};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::{Affine3A, Dir3, Mat3, Mat4, Vec3};
use bevy_reflect::prelude::*;
use bevy_transform::components::{GlobalTransform, Transform};

use crate::cluster::{ClusterVisibilityClass, GlobalVisibleClusterableObjects};

/// A light that emits light in a given direction from a central point.
///
/// Behaves like a point light in a perfectly absorbent housing that
/// shines light only in a given direction. The direction is taken from
/// the transform, and can be specified with [`Transform::looking_at`](Transform::looking_at).
///
/// To control the resolution of the shadow maps, use the [`crate::DirectionalLightShadowMap`] resource.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Frustum, VisibleMeshEntities, Transform, Visibility, VisibilityClass)]
#[component(on_add = visibility::add_visibility_class::<ClusterVisibilityClass>)]
pub struct SpotLight {
    /// The color of the light.
    ///
    /// By default, this is white.
    pub color: Color,

    /// Luminous power in lumens, representing the amount of light emitted by this source in all directions.
    pub intensity: f32,

    /// Range in meters that this light illuminates.
    ///
    /// Note that this value affects resolution of the shadow maps; generally, the
    /// higher you set it, the lower-resolution your shadow maps will be.
    /// Consequently, you should set this value to be only the size that you need.
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
    ///
    /// Note that shadows are rather expensive and become more so with every
    /// light that casts them. In general, it's best to aggressively limit the
    /// number of lights with shadows enabled to one or two at most.
    pub shadows_enabled: bool,

    /// Whether soft shadows are enabled.
    ///
    /// Soft shadows, also known as *percentage-closer soft shadows* or PCSS,
    /// cause shadows to become blurrier (i.e. their penumbra increases in
    /// radius) as they extend away from objects. The blurriness of the shadow
    /// depends on the [`SpotLight::radius`] of the light; larger lights result in larger
    /// penumbras and therefore blurrier shadows.
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

    /// Whether this spot light contributes diffuse lighting to meshes with
    /// lightmaps.
    ///
    /// Set this to false if your lightmap baking tool bakes the direct diffuse
    /// light from this directional light into the lightmaps in order to avoid
    /// counting the radiance from this light twice. Note that the specular
    /// portion of the light is always considered, because Bevy currently has no
    /// means to bake specular light.
    ///
    /// By default, this is set to true.
    pub affects_lightmapped_mesh_diffuse: bool,

    /// A value that adjusts the tradeoff between self-shadowing artifacts and
    /// proximity of shadows to their casters.
    ///
    /// This value frequently must be tuned to the specific scene; this is
    /// normal and a well-known part of the shadow mapping workflow. If set too
    /// low, unsightly shadow patterns appear on objects not in shadow as
    /// objects incorrectly cast shadows on themselves, known as *shadow acne*.
    /// If set too high, shadows detach from the objects casting them and seem
    /// to "fly" off the objects, known as *Peter Panning*.
    pub shadow_depth_bias: f32,

    /// A bias applied along the direction of the fragment's surface normal. It is scaled to the
    /// shadow map's texel size so that it can be small close to the camera and gets larger further
    /// away.
    pub shadow_normal_bias: f32,

    /// The distance from the light to the near Z plane in the shadow map.
    ///
    /// Objects closer than this distance to the light won't cast shadows.
    /// Setting this higher increases the shadow map's precision.
    ///
    /// This only has an effect if shadows are enabled.
    pub shadow_map_near_z: f32,

    /// Angle defining the distance from the spot light direction to the outer limit
    /// of the light's cone of effect.
    /// `outer_angle` should be < `PI / 2.0`.
    /// `PI / 2.0` defines a hemispherical spot light, but shadows become very blocky as the angle
    /// approaches this limit.
    pub outer_angle: f32,

    /// Angle defining the distance from the spot light direction to the inner limit
    /// of the light's cone of effect.
    /// Light is attenuated from `inner_angle` to `outer_angle` to give a smooth falloff.
    /// `inner_angle` should be <= `outer_angle`
    pub inner_angle: f32,
}

impl SpotLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 1.8;
    pub const DEFAULT_SHADOW_MAP_NEAR_Z: f32 = 0.1;
}

impl Default for SpotLight {
    fn default() -> Self {
        // a quarter arc attenuating from the center
        Self {
            color: Color::WHITE,
            // 1,000,000 lumens is a very large "cinema light" capable of registering brightly at Bevy's
            // default "very overcast day" exposure level. For "indoor lighting" with a lower exposure,
            // this would be way too bright.
            intensity: 1_000_000.0,
            range: 20.0,
            radius: 0.0,
            shadows_enabled: false,
            affects_lightmapped_mesh_diffuse: true,
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
            shadow_map_near_z: Self::DEFAULT_SHADOW_MAP_NEAR_Z,
            inner_angle: 0.0,
            outer_angle: core::f32::consts::FRAC_PI_4,
            #[cfg(feature = "experimental_pbr_pcss")]
            soft_shadows_enabled: false,
        }
    }
}

/// Constructs a right-handed orthonormal basis from a given unit Z vector.
///
/// This method of constructing a basis from a [`Vec3`] is used by [`bevy_math::Vec3::any_orthonormal_pair`]
// we will also construct it in the fragment shader and need our implementations to match exactly,
// so we reproduce it here to avoid a mismatch if glam changes.
// See bevy_render/maths.wgsl:orthonormalize
pub fn orthonormalize(z_basis: Dir3) -> Mat3 {
    let sign = 1f32.copysign(z_basis.z);
    let a = -1.0 / (sign + z_basis.z);
    let b = z_basis.x * z_basis.y * a;
    let x_basis = Vec3::new(
        1.0 + sign * z_basis.x * z_basis.x * a,
        sign * b,
        -sign * z_basis.x,
    );
    let y_basis = Vec3::new(b, sign + z_basis.y * z_basis.y * a, -z_basis.y);
    Mat3::from_cols(x_basis, y_basis, z_basis.into())
}
/// Constructs a right-handed orthonormal basis with translation, using only the forward direction and translation of a given [`GlobalTransform`].
///
/// This is a version of [`orthonormalize`] which also includes translation.
pub fn spot_light_world_from_view(transform: &GlobalTransform) -> Affine3A {
    // the matrix z_local (opposite of transform.forward())
    let fwd_dir = transform.back();

    let basis = orthonormalize(fwd_dir);
    Affine3A::from_mat3_translation(basis, transform.translation())
}

pub fn spot_light_clip_from_view(angle: f32, near_z: f32) -> Mat4 {
    // spot light projection FOV is 2x the angle from spot light center to outer edge
    Mat4::perspective_infinite_reverse_rh(angle * 2.0, 1.0, near_z)
}

/// Add to a [`SpotLight`] to add a light texture effect.
/// A texture mask is applied to the light source to modulate its intensity,  
/// simulating patterns like window shadows, gobo/cookie effects, or soft falloffs.
#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Component, Debug)]
#[require(SpotLight)]
pub struct SpotLightTexture {
    /// The texture image. Only the R channel is read.
    /// Note the border of the image should be entirely black to avoid leaking light.
    pub image: Handle<Image>,
}

pub fn update_spot_light_frusta(
    global_lights: Res<GlobalVisibleClusterableObjects>,
    mut views: Query<
        (Entity, &GlobalTransform, &SpotLight, &mut Frustum),
        Or<(Changed<GlobalTransform>, Changed<SpotLight>)>,
    >,
) {
    for (entity, transform, spot_light, mut frustum) in &mut views {
        // The frusta are used for culling meshes to the light for shadow mapping
        // so if shadow mapping is disabled for this light, then the frusta are
        // not needed.
        // Also, if the light is not relevant for any cluster, it will not be in the
        // global lights set and so there is no need to update its frusta.
        if !spot_light.shadows_enabled || !global_lights.entities.contains(&entity) {
            continue;
        }

        // ignore scale because we don't want to effectively scale light radius and range
        // by applying those as a view transform to shadow map rendering of objects
        let view_backward = transform.back();

        let spot_world_from_view = spot_light_world_from_view(transform);
        let spot_clip_from_view =
            spot_light_clip_from_view(spot_light.outer_angle, spot_light.shadow_map_near_z);
        let clip_from_world = spot_clip_from_view * spot_world_from_view.inverse();

        *frustum = Frustum::from_clip_from_world_custom_far(
            &clip_from_world,
            &transform.translation(),
            &view_backward,
            spot_light.range,
        );
    }
}
