use bevy_ecs::Entity;
use bevy_property::Properties;
use std::ops::{DerefMut, Deref};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Properties)]
pub struct Parent(pub Entity);

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PreviousParent(pub Option<Entity>);

impl Deref for Parent {
    type Target = Entity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Parent {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}