use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMapper, MapEntities},
    reflect::{ReflectComponent, ReflectMapEntities},
    world::{FromWorld, World},
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
#[derive(Component, Debug, Eq, PartialEq, Reflect)]
#[reflect(Component, MapEntities, PartialEq)]
pub struct Parent(pub(crate) Entity);

impl Parent {
    /// Gets the [`Entity`] ID of the parent.
    pub fn get(&self) -> Entity {
        self.0
    }

    /// Gets the parent [`Entity`] as a slice of length 1.
    ///
    /// Useful for making APIs that require a type or homogeneous storage
    /// for both [`Children`] & [`Parent`] that is agnostic to edge direction.
    ///
    /// [`Children`]: super::children::Children
    pub fn as_slice(&self) -> &[Entity] {
        std::slice::from_ref(&self.0)
    }
}

// TODO: We need to impl either FromWorld or Default so Parent can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Parent should only ever be set with a real user-defined entity.  Its worth looking into
// better ways to handle cases like this.
impl FromWorld for Parent {
    fn from_world(_world: &mut World) -> Self {
        Parent(Entity::PLACEHOLDER)
    }
}

impl MapEntities for Parent {
    fn map_entities(&mut self, entity_mapper: &mut EntityMapper) {
        self.0 = entity_mapper.get_or_reserve(self.0);
    }
}

impl Deref for Parent {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
