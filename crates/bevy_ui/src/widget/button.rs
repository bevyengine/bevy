use bevy_ecs::prelude::Component;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::std_traits::ReflectDefault;
use bevy_reflect::Reflect;

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default)]
pub struct Button;
