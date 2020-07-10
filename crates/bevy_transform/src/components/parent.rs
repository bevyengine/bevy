use bevy_ecs::Entity;
use bevy_property::Properties;
use shrinkwraprs::Shrinkwrap;

#[derive(Shrinkwrap, Debug, Copy, Clone, Eq, PartialEq, Properties)]
#[shrinkwrap(mutable)]
pub struct Parent(pub Entity);

#[derive(Shrinkwrap, Debug, Copy, Clone, Eq, PartialEq)]
#[shrinkwrap(mutable)]
pub struct PreviousParent(pub Option<Entity>);
