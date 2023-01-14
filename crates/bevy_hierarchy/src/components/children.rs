use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMap, MapEntities, MapEntitiesError},
    prelude::FromWorld,
    reflect::{ReflectComponent, ReflectMapEntities},
    world::World,
};
use bevy_reflect::Reflect;
use core::slice;
use smallvec::SmallVec;
use std::ops::Deref;

/// Contains references to the child entities of this entity.
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
#[derive(Component, Debug, Reflect)]
#[reflect(Component, MapEntities)]
pub struct Children(pub(crate) SmallVec<[Entity; 8]>);

impl MapEntities for Children {
    fn map_entities(&mut self, entity_map: &EntityMap) -> Result<(), MapEntitiesError> {
        for entity in &mut self.0 {
            *entity = entity_map.get(*entity)?;
        }

        Ok(())
    }
}

// TODO: We need to impl either FromWorld or Default so Children can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Children should only ever be set with a real user-defined entities. Its worth looking
// into better ways to handle cases like this.
impl FromWorld for Children {
    fn from_world(_world: &mut World) -> Self {
        Children(SmallVec::new())
    }
}

impl Children {
    /// Constructs a [`Children`] component with the given entities.
    pub(crate) fn from_entities(entities: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entities))
    }

    /// Swaps the child at `a_index` with the child at `b_index`.
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

impl<'a> IntoIterator for &'a Children {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = slice::Iter<'a, Entity>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
