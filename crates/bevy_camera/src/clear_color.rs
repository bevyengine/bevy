use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use derive_more::derive::From;
use serde::{Deserialize, Serialize};

/// For a camera, specifies the color used to clear the viewport
/// [before rendering](crate::camera::Camera::clear_color)
/// or when [writing to the final render target texture](crate::camera::Camera::output_mode).
#[derive(Reflect, Serialize, Deserialize, Copy, Clone, Debug, Default, From)]
#[reflect(Serialize, Deserialize, Default, Clone)]
pub enum ClearColorConfig {
    /// The clear color is taken from the world's [`ClearColor`] resource.
    #[default]
    Default,
    /// The given clear color is used, overriding the [`ClearColor`] resource defined in the world.
    Custom(Color),
    /// No clear color is used: the camera will simply draw on top of anything already in the viewport.
    ///
    /// This can be useful when multiple cameras are rendering to the same viewport.
    None,
}

/// A [`Resource`] that stores the default color that cameras use to clear the screen between frames.
///
/// This color appears as the "background" color for simple apps,
/// when there are portions of the screen with nothing rendered.
///
/// Individual cameras may use [`Camera.clear_color`] to specify a different
/// clear color or opt out of clearing their viewport.
///
/// [`Camera.clear_color`]: crate::camera::Camera::clear_color
#[derive(Resource, Clone, Debug, Deref, DerefMut, Reflect)]
#[reflect(Resource, Default, Debug, Clone)]
pub struct ClearColor(pub Color);

/// Match the dark gray bevy website code block color by default.
impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::srgb_u8(43, 44, 47))
    }
}
