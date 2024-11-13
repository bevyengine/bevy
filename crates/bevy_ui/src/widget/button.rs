use crate::{FocusPolicy, Interaction, Node};
use bevy_ecs::{
    prelude::Component,
    reflect::{ReflectComponent, ReflectComponentMut},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(ComponentMut, Component, Default, Debug, PartialEq)]
#[require(Node, FocusPolicy(|| FocusPolicy::Block), Interaction)]
pub struct Button;
