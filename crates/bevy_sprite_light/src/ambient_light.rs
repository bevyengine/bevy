use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;

/// Light applied to every 2d entity.
///
/// The default is white at full brightness, so the brightness needs to be turned down in order
/// for other light sources to take effect.
#[derive(Resource, Clone, Debug, Reflect)]
#[reflect(Resource, Debug, Default, Clone)]
pub struct GlobalAmbientLight2d {
    /// The color of the ambient light.
    pub color: Color,
    /// The brightness of the ambient light. `1.0` is full brightness. `0.0` will result in a
    /// completely dark scene.
    pub brightness: f32,
}

impl Default for GlobalAmbientLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            brightness: 1.0,
        }
    }
}
