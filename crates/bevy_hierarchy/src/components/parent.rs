#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{
    ReflectComponent, ReflectFromWorld, ReflectMapEntities, ReflectVisitEntities,
    ReflectVisitEntitiesMut,
};
use bevy_ecs::{
    component::{Component, ComponentCloneHandler, Mutable, StorageType},
    entity::{Entity, VisitEntities, VisitEntitiesMut},
    traversal::Traversal,
    world::{FromWorld, World},
};
use core::ops::Deref;

/// Holds a reference to the parent entity of this entity.
/// This component should only be present on entities that actually have a parent entity.
///
/// Parent entity must have this entity stored in its [`Children`] component.
/// It is hard to set up parent/child relationships manually,
/// consider using higher level utilities like [`BuildChildren::with_children`].
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`Children`]: super::children::Children
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
#[derive(Debug, Eq, PartialEq, VisitEntities, VisitEntitiesMut)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(
    feature = "reflect",
    reflect(
        Component,
        MapEntities,
        VisitEntities,
        VisitEntitiesMut,
        PartialEq,
        Debug,
        FromWorld
    )
)]
pub struct Parent(pub(crate) Entity);

impl Component for Parent {
    const STORAGE_TYPE: StorageType = StorageType::Table;
    type Mutability = Mutable;

    fn get_component_clone_handler() -> ComponentCloneHandler {
        ComponentCloneHandler::ignore()
    }
}

impl Parent {
    /// Gets the [`Entity`] ID of the parent.
    #[inline(always)]
    pub fn get(&self) -> Entity {
        self.0
    }

    /// Gets the parent [`Entity`] as a slice of length 1.
    ///
    /// Useful for making APIs that require a type or homogeneous storage
    /// for both [`Children`] & [`Parent`] that is agnostic to edge direction.
    ///
    /// [`Children`]: super::children::Children
    #[inline(always)]
    pub fn as_slice(&self) -> &[Entity] {
        core::slice::from_ref(&self.0)
    }
}

// TODO: We need to impl either FromWorld or Default so Parent can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Parent should only ever be set with a real user-defined entity.  Its worth looking into
// better ways to handle cases like this.
impl FromWorld for Parent {
    #[inline(always)]
    fn from_world(_world: &mut World) -> Self {
        Parent(Entity::PLACEHOLDER)
    }
}

impl Deref for Parent {
    type Target = Entity;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// This provides generalized hierarchy traversal for use in [event propagation].
///
/// `Parent::traverse` will never form loops in properly-constructed hierarchies.
///
/// [event propagation]: bevy_ecs::observer::Trigger::propagate
impl<D> Traversal<D> for &Parent {
    fn traverse(item: Self::Item<'_>, _data: &D) -> Option<Entity> {
        Some(item.0)
    }
}
