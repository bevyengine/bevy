use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::{FromReflect, Reflect, ReflectFromReflect};

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, Reflect, FromReflect)]
#[reflect(Component, FromReflect, Default)]
pub struct Button;
