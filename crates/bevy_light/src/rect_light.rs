use bevy_camera::visibility::{Visibility, VisibilityClass};
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_transform::components::Transform;

use crate::light_consts;

/// A rectangular area light.
///
/// The rectangle lies in the XY plane of the entity's local coordinate frame
/// and faces the local -Z direction.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Transform, Visibility, VisibilityClass)]
pub struct RectLight {
    /// The color of the light.
    ///
    /// By default, this is white.
    pub color: Color,

    /// Luminous power in lumens, representing the amount of light emitted by this source in all directions.
    pub intensity: f32,

    /// Width of the light rectangle (along local X).
    pub width: f32,

    /// Height of the light rectangle (along local Y).
    pub height: f32,
}

impl Default for RectLight {
    fn default() -> Self {
        RectLight {
            color: Color::WHITE,
            intensity: light_consts::lumens::VERY_LARGE_CINEMA_LIGHT,
            width: 1.0,
            height: 1.0,
        }
    }
}
