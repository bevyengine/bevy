use crate::ecs::prelude::*;
use shrinkwraprs::Shrinkwrap;
use bevy_property::Properties;

#[derive(Shrinkwrap, Debug, Copy, Clone, Eq, PartialEq, Properties)]
#[shrinkwrap(mutable)]
pub struct Parent(pub Entity);

#[derive(Shrinkwrap, Debug, Copy, Clone, Eq, PartialEq)]
#[shrinkwrap(mutable)]
pub struct PreviousParent(pub Option<Entity>);
