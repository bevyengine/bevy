use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_reflect::{Reflect, ReflectDeserialize, ReflectSerialize};
use bevy_render::{color::Color, extract_resource::ExtractResource};
use serde::{Deserialize, Serialize};

/// For a camera, specifies the color used to clear the viewport before rendering.
#[derive(Reflect, Serialize, Deserialize, Clone, Debug, Default)]
#[reflect(Serialize, Deserialize)]
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

/// A [`Resource`] that stores the color that is used to clear the screen between frames.
///
/// This color appears as the "background" color for simple apps,
/// when there are portions of the screen with nothing rendered.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource, Reflect)]
#[reflect(Resource)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::rgb(0.4, 0.4, 0.4))
    }
}
