use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    reflect::{ReflectComponent, ReflectMapEntities},
    world::{FromWorld, World},
};
use bevy_reflect::Reflect;
use std::ops::{Deref, DerefMut};

#[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct Parent(pub Entity);

// TODO: We need to impl either FromWorld or Default so Parent can be registered as Properties.
// This is because Properties deserialize by creating an instance and apply a patch on top.
// However Parent should only ever be set with a real user-defined entity.  Its worth looking into
// better ways to handle cases like this.
impl FromWorld for Parent {
    fn from_world(_world: &mut World) -> Self {
        Parent(Entity::new(u32::MAX))
    }
}

impl MapEntities for Parent {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

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

#[derive(Component, Debug, Copy, Clone, Eq, PartialEq, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct PreviousParent(pub(crate) Entity);

impl MapEntities for PreviousParent {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

// TODO: Better handle this case see `impl FromWorld for Parent`
impl FromWorld for PreviousParent {
    fn from_world(_world: &mut World) -> Self {
        PreviousParent(Entity::new(u32::MAX))
    }
}
