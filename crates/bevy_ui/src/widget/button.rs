use crate::{FocusPolicy, Interaction, Node};
use bevy_ecs::{
    prelude::{Component, require},
    reflect::ReflectComponent,
};
use bevy_reflect::{Reflect, std_traits::ReflectDefault};

/// Marker struct for buttons
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq)]
#[require(Node, FocusPolicy(|| FocusPolicy::Block), Interaction)]
pub struct Button;
