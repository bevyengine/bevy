use bevy_render::view::{self, Visibility};

use super::*;

/// A light that emits light in a given direction from a central point.
///
/// Behaves like a point light in a perfectly absorbent housing that
/// shines light only in a given direction. The direction is taken from
/// the transform, and can be specified with [`Transform::looking_at`](Transform::looking_at).
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Frustum, VisibleMeshEntities, Transform, Visibility, VisibilityClass)]
#[component(on_add = view::add_visibility_class::<LightVisibilityClass>)]
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
