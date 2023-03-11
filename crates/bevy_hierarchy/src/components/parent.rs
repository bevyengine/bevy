use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    reflect::{ReflectComponent, ReflectMapEntities},
};
use bevy_reflect::Reflect;
use std::ops::Deref;

/// Holds a reference to the parent entity of this entity.
/// This component should only be present on entities that actually have a parent entity.
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
#[derive(Component, Debug, Default, Eq, PartialEq, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct Parent(Option<Entity>);

impl Parent {

    pub(crate) fn new(entity: Entity) -> Self {
        Self(Some(entity))
    }

    /// Gets the [`Entity`] ID of the parent.
    pub fn try_get(&self) -> Option<Entity> {
        self.0
    }
}

impl MapEntities for Parent {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        // Parent of an entity in the new world can be in outside world, in which case it
        // should not be mapped.
        if let Some(entity) = self.0 {
            if let Ok(mapped_entity) = entity_map.get(entity) {
                self.0 = Some(mapped_entity);
            }
        }
        Ok(())
    }
}

impl Deref for Parent {
    type Target = Option<Entity>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
