use bevy_ecs::Entity;
use bevy_property::Properties;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Properties)]
pub struct Parent(pub Entity);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PreviousParent(pub Option<Entity>);
