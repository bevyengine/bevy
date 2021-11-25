use bevy_ecs::prelude::Component;
use bevy_reflect::Reflect;
use bevy_ecs::reflect::ReflectComponent;

#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct Button;
