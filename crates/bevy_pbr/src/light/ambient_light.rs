use super::*;

/// An ambient light, which lights the entire scene equally.
///
/// This resource is inserted by the [`PbrPlugin`] and by default it is set to a low ambient light.
///
/// # Examples
///
/// Make ambient light slightly brighter:
///
/// ```
/// # use bevy_ecs::system::ResMut;
/// # use bevy_pbr::AmbientLight;
/// fn setup_ambient_light(mut ambient_light: ResMut<AmbientLight>) {
///    ambient_light.brightness = 100.0;
/// }
/// ```
#[derive(Resource, Clone, Debug, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct AmbientLight {
    pub color: Color,
    /// A direct scale factor multiplied with `color` before being passed to the shader.
    ///
    /// After applying this multiplier, the resulting value should be in units of [cd/m^2].
    ///
    /// [cd/m^2]: https://en.wikipedia.org/wiki/Candela_per_square_metre
    pub brightness: f32,
    /// The degree to which this light is monochromatic. A value of 0.0 means this light is perfectly polychromatic,
    /// while a value of 1.0 means this light is perfectly monochromatic.
    ///
    /// Meant to be used with `SpectralColor` to render monochromatic lights. (e.g. Sodium vapor lamps)
    /// Combining non-zero values with non-spectral colors is not physically correct, but can be used for artistic effect.
    #[cfg(feature = "spectral_lighting")]
    pub monochromaticity: f32,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            brightness: 80.0,
            #[cfg(feature = "spectral_lighting")]
            monochromaticity: 0.0,
        }
    }
}
impl AmbientLight {
    pub const NONE: AmbientLight = AmbientLight {
        color: Color::WHITE,
        brightness: 0.0,
        #[cfg(feature = "spectral_lighting")]
        monochromaticity: 0.0,
    };
}
