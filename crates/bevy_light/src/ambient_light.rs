use bevy_camera::Camera;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// An ambient light, which lights the entire scene equally.
///
/// It can be added to a camera to override [`GlobalAmbientLight`], which is the default that is otherwise used.
#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Debug, Default, Clone)]
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

/// The global ambient light, which lights the entire scene equally.
///
/// This resource is inserted by the [`LightPlugin`] and by default it is set to a low ambient light.
/// Inserting an [`AmbientLight`] on a camera will override this default.
///
/// # Examples
///
/// Make ambient light slightly brighter:
///
/// ```
/// # use bevy_ecs::system::ResMut;
/// # use bevy_light::GlobalAmbientLight;
/// fn setup_ambient_light(mut ambient_light: ResMut<GlobalAmbientLight>) {
///    ambient_light.brightness = 100.0;
/// }
/// ```
///
/// [`LightPlugin`]: crate::LightPlugin
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource, Debug, Default, Clone)]
pub struct GlobalAmbientLight {
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

impl Default for GlobalAmbientLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            brightness: 80.0,
            affects_lightmapped_meshes: true,
        }
    }
}

impl GlobalAmbientLight {
    pub const NONE: GlobalAmbientLight = GlobalAmbientLight {
        color: Color::WHITE,
        brightness: 0.0,
        affects_lightmapped_meshes: true,
    };
}
