//! Define the relationship between attaching and attached models

use alloc::vec::Vec;

use bevy_derive::Deref;
use bevy_ecs::{component::Component, entity::Entity};
use bevy_reflect::Reflect;

#[derive(Debug, Component, Reflect)]
#[relationship_target(relationship = AttachedTo)]
/// List of models attached to this model
pub struct AttachingModels(Vec<Entity>);

#[derive(Debug, Component, Reflect, Deref)]
#[relationship(relationship_target=AttachingModels)]
/// Model this entity is attached to
pub struct AttachedTo(Entity);

impl From<Entity> for AttachedTo {
    fn from(entity: Entity) -> Self {
        Self(entity)
    }
}
