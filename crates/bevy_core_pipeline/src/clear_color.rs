use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_reflect::{
    FromReflect, Reflect, ReflectDeserialize, ReflectFromReflect, ReflectSerialize,
};
use bevy_render::{color::Color, extract_resource::ExtractResource};
use serde::{Deserialize, Serialize};

#[derive(Reflect, FromReflect, Serialize, Deserialize, Clone, Debug, Default)]
#[reflect(Serialize, Deserialize, FromReflect)]
pub enum ClearColorConfig {
    #[default]
    Default,
    Custom(Color),
    None,
}

/// A [`Resource`] that stores the color that is used to clear the screen between frames.
///
/// This color appears as the "background" color for simple apps,
/// when there are portions of the screen with nothing rendered.
#[derive(Resource, Clone, Debug, Deref, DerefMut, ExtractResource, Reflect, FromReflect)]
#[reflect(Resource, FromReflect)]
pub struct ClearColor(pub Color);

impl Default for ClearColor {
    fn default() -> Self {
        Self(Color::rgb(0.4, 0.4, 0.4))
    }
}
