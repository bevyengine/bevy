use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    reflect::{ReflectComponent, ReflectMapEntities},
};
use bevy_reflect::Reflect;
use smallvec::SmallVec;
use std::ops::Deref;

#[derive(Component, Default, Clone, Debug, Reflect)]
#[reflect(Component, MapEntities)]
pub struct Children(pub(crate) SmallVec<[Entity; 8]>);

impl MapEntities for Children {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        for entity in self.0.iter_mut() {
            *entity = entity_map.get(*entity)?;
        }

        Ok(())
    }
}

impl Children {
    pub fn with(entity: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entity))
    }

    /// Swaps the child at `a_index` with the child at `b_index`
    pub fn swap(&mut self, a_index: usize, b_index: usize) {
        self.0.swap(a_index, b_index);
    }
}

impl Deref for Children {
    type Target = [Entity];

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}
