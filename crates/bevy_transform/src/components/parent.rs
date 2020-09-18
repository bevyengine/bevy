use bevy_ecs::{Entity, FromResources};
use bevy_property::Properties;
use std::ops::{Deref, DerefMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Properties)]
pub struct Parent(pub Entity);

// TODO: We need to impl either FromResources or Default so Parent can be registered as Properties.
// This is because Properties deserialize by creating an instance and apply a patch on top.
// However Parent should only ever be set with a real user-defined entity.  Its worth looking into better
// ways to handle cases like this.
impl FromResources for Parent {
    fn from_resources(_resources: &bevy_ecs::Resources) -> Self {
        Parent(Entity::new(u32::MAX))
    }
}

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
