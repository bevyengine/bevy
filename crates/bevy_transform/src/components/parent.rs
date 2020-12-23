use bevy_ecs::{Entity, FromResources, MapEntities};
use bevy_reflect::{Reflect, ReflectComponent, ReflectMapEntities};
use std::ops::{Deref, DerefMut};

#[derive(Debug, Copy, Clone, Eq, PartialEq, Reflect)]
#[reflect(Component, MapEntities)]
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

impl MapEntities for Parent {
    fn map_entities(
        &mut self,
        entity_map: &bevy_ecs::EntityMap,
    ) -> Result<(), bevy_ecs::MapEntitiesError> {
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

#[derive(Debug, Copy, Clone, Eq, PartialEq, Reflect)]
#[reflect(Component, MapEntities)]
pub struct PreviousParent(pub(crate) Entity);

impl MapEntities for PreviousParent {
    fn map_entities(
        &mut self,
        entity_map: &bevy_ecs::EntityMap,
    ) -> Result<(), bevy_ecs::MapEntitiesError> {
        self.0 = entity_map.get(self.0)?;
        Ok(())
    }
}

// TODO: Better handle this case see `impl FromResources for Parent`
impl FromResources for PreviousParent {
    fn from_resources(_resources: &bevy_ecs::Resources) -> Self {
        PreviousParent(Entity::new(u32::MAX))
    }
}
