use crate::{FocusPolicy, Interaction, Node};
use bevy_ecs::{
    prelude::{require, Component},
    reflect::ReflectComponent,
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// A marker struct for buttons.
///
/// Buttons should use an observer to listen for the [`Activate`](crate::Activate) action to perform their primary action.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Component, Default, Debug, PartialEq)]
#[require(Node, FocusPolicy(|| FocusPolicy::Block), Interaction)]
pub struct Button;
