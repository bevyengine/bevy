use bevy_camera::Camera;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// An ambient light, which lights the entire scene equally.
///
/// This resource is inserted by the [`LightPlugin`] and by default it is set to a low ambient light.
///
/// It can also be added to a camera to override the resource (or default) ambient for that camera only.
///
/// # Examples
///
/// Make ambient light slightly brighter:
///
/// ```
/// # use bevy_ecs::system::ResMut;
/// # use bevy_light::AmbientLight;
/// fn setup_ambient_light(mut ambient_light: ResMut<AmbientLight>) {
///    ambient_light.brightness = 100.0;
/// }
/// ```
///
/// [`LightPlugin`]: crate::LightPlugin
#[derive(Resource, Component, Clone, Debug, Reflect)]
#[reflect(Resource, Component, Debug, Default, Clone)]
#[require(Camera)]
pub struct AmbientLight {
    pub color: Color,

    /// A direct scale factor multiplied with `color` before being passed to the shader.
    ///
    /// After applying this multiplier, the resulting value should be in units of [cd/m^2].
    ///
    /// [cd/m^2]: https://en.wikipedia.org/wiki/Candela_per_square_metre
    pub brightness: f32,

    /// Whether this ambient light has an effect on meshes with lightmaps.
    ///
    /// Set this to false if your lightmap baking tool bakes the ambient light
    /// into the lightmaps, to avoid rendering that light twice.
    ///
    /// By default, this is set to true.
    pub affects_lightmapped_meshes: bool,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            brightness: 80.0,
            affects_lightmapped_meshes: true,
        }
    }
}

impl AmbientLight {
    pub const NONE: AmbientLight = AmbientLight {
        color: Color::WHITE,
        brightness: 0.0,
        affects_lightmapped_meshes: true,
    };
}
