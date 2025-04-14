use bevy_render::view::{self, Visibility};

use super::*;

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
#[component(on_add = view::add_visibility_class::<LightVisibilityClass>)]
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
