use bevy_ecs::{
    prelude::Component,
    reflect::{ReflectComponent, ReflectComponentMut},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// Marker struct for labels
#[derive(Component, Debug, Default, Clone, Copy, Reflect)]
#[reflect(ComponentMut, Component, Default, Debug)]
pub struct Label;
