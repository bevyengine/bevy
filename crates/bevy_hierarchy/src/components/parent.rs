#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectMapEntities};
use bevy_ecs::{
    component::Component,
    entity::{Entity, EntityMapper, MapEntities},
    world::{FromWorld, World},
};
use std::ops::Deref;

/// Component referencing the parent entity.
///
/// To get the parent [`Entity`], call the [`get`] method.
///
/// This component is automatically removed once the entity loses its parent.
///
/// Check the [crate-level documentation]
/// to learn how to correctly use this component.
///
/// [crate-level documentation]: crate
/// [`get`]: Self::get
#[derive(Component, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "reflect", reflect(Component, MapEntities, PartialEq))]
pub struct Parent(pub(crate) Entity);

impl Parent {
    /// Returns the parent [`Entity`].
    pub fn get(&self) -> Entity {
        self.0
    }

    /// Returns the parent [`Entity`] as a slice of length `1`.
    ///
    /// Useful for making APIs that require a type or homogeneous storage
    /// for both [`Children`] and [`Parent`] that is agnostic to edge direction.
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
