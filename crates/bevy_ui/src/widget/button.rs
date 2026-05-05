use crate::{FocusPolicy, Interaction, Node};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[require(Node, FocusPolicy::Block, Interaction)]
pub struct Button;
