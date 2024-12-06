use bevy_render::view::Visibility;

use super::*;

/// A Directional light.
///
/// Directional lights don't exist in reality but they are a good
/// approximation for light sources VERY far away, like the sun or
/// the moon.
///
/// The light shines along the forward direction of the entity's transform. With a default transform
/// this would be along the negative-Z axis.
///
/// Valid values for `illuminance` are:
///
/// | Illuminance (lux) | Surfaces illuminated by                        |
/// |-------------------|------------------------------------------------|
/// | 0.0001            | Moonless, overcast night sky (starlight)       |
/// | 0.002             | Moonless clear night sky with airglow          |
/// | 0.05–0.3          | Full moon on a clear night                     |
/// | 3.4               | Dark limit of civil twilight under a clear sky |
/// | 20–50             | Public areas with dark surroundings            |
/// | 50                | Family living room lights                      |
/// | 80                | Office building hallway/toilet lighting        |
/// | 100               | Very dark overcast day                         |
/// | 150               | Train station platforms                        |
/// | 320–500           | Office lighting                                |
/// | 400               | Sunrise or sunset on a clear day.              |
/// | 1000              | Overcast day; typical TV studio lighting       |
/// | 10,000–25,000     | Full daylight (not direct sun)                 |
/// | 32,000–100,000    | Direct sunlight                                |
///
/// Source: [Wikipedia](https://en.wikipedia.org/wiki/Lux)
///
/// ## Shadows
///
/// To enable shadows, set the `shadows_enabled` property to `true`.
///
/// Shadows are produced via [cascaded shadow maps](https://developer.download.nvidia.com/SDK/10.5/opengl/src/cascaded_shadow_maps/doc/cascaded_shadow_maps.pdf).
///
/// To modify the cascade setup, such as the number of cascades or the maximum shadow distance,
/// change the [`CascadeShadowConfig`] component of the entity with the [`DirectionalLight`].
///
/// To control the resolution of the shadow maps, use the [`DirectionalLightShadowMap`] resource:
///
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_pbr::DirectionalLightShadowMap;
/// App::new()
///     .insert_resource(DirectionalLightShadowMap { size: 2048 });
/// ```
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default, Debug)]
#[require(
    Cascades,
    CascadesFrusta,
    CascadeShadowConfig,
    CascadesVisibleEntities,
    LightShadows,
    Transform,
    Visibility
)]
pub struct DirectionalLight {
    /// The color of the light.
    ///
    /// By default, this is white.
    pub color: Color,

    /// Illuminance in lux (lumens per square meter), representing the amount of
    /// light projected onto surfaces by this light source. Lux is used here
    /// instead of lumens because a directional light illuminates all surfaces
    /// more-or-less the same way (depending on the angle of incidence). Lumens
    /// can only be specified for light sources which emit light from a specific
    /// area.
    pub illuminance: f32,

    /// The angular size of this light in radians. This must be a value in the 
    /// range [0, π).
    ///
    /// The angular size of an object is how large it appears in one's field
    /// of view. For example, if viewed from the surface of the Earth, if one
    /// were to draw a line (specifically a *great circle*]) across the sky
    /// through the center of the Sun, the Sun would cover roughly 0.5 degrees
    /// of that circle, or 8.73e-3 radians.
    ///
    /// This value controls the radius of soft shadow penumbras, as well as
    /// some volumetric lighting effects. See [`LightShadows`](crate::light::LightShadows)
    /// for more information on soft shadows.
    ///
    /// Note that this is not the same thing as the *solid angle* (or "angular
    /// area", roughly) that this light covers in the sky. That is a separate
    /// measurement, with units of *steradians* rather than radians.
    pub angular_size: f32,

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

    /// A bias applied along the direction of the fragment's surface normal. It
    /// is scaled to the shadow map's texel size so that it is automatically
    /// adjusted to the orthographic projection.
    pub shadow_normal_bias: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        DirectionalLight {
            color: Color::WHITE,
            illuminance: light_consts::lux::AMBIENT_DAYLIGHT,
            angular_size: Self::SUN_ANGULAR_SIZE,
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
        }
    }
}

impl DirectionalLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 1.8;
    pub const SUN_ANGULAR_SIZE: f32 = 8.72665e-3;
}
