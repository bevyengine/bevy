#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectMapEntities};
use bevy_ecs::{
    archetype::Archetype,
    component::{Component, ComponentId, Components, Tick},
    entity::{Entity, EntityMapper, MapEntities},
    query::{FilteredAccess, QueryData, ReadFetch, ReadOnlyQueryData, WorldQuery},
    storage::{Table, TableRow},
    traversal::Traversal,
    world::{unsafe_world_cell::UnsafeWorldCell, FromWorld, World},
};
use std::ops::Deref;

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
#[derive(Component, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "reflect", reflect(Component, MapEntities, PartialEq))]
pub struct Parent(pub(crate) Entity);

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
        std::slice::from_ref(&self.0)
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

impl MapEntities for Parent {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        self.0 = entity_mapper.map_entity(self.0);
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
impl Traversal for Parent {
    fn traverse(&self) -> Option<Entity> {
        Some(self.0)
    }
}

#[allow(unsafe_code)]
/// SAFETY:
/// This implementation delegates to the existing implementation for &Parent
unsafe impl WorldQuery for Parent
where
    Self: Component,
{
    type Item<'w> = Entity;
    type Fetch<'w> = ReadFetch<'w, Self>;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        <&Self as WorldQuery>::shrink_fetch(fetch)
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        state: &Self::State,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w> {
        // SAFETY: This implementation delegates to the existing implementation for &Self
        unsafe { <&Self as WorldQuery>::init_fetch(world, state, last_run, this_run) }
    }

    const IS_DENSE: bool = <&Self as WorldQuery>::IS_DENSE;

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        // SAFETY: This implementation delegates to the existing implementation for &Self
        unsafe { <&Self as WorldQuery>::set_archetype(fetch, state, archetype, table) }
    }

    #[inline]
    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table) {
        // SAFETY: This implementation delegates to the existing implementation for &Self
        unsafe { <&Self as WorldQuery>::set_table(fetch, state, table) }
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: This implementation delegates to the existing implementation for &Self
        unsafe { <&Self as WorldQuery>::fetch(fetch, entity, table_row).get() }
    }

    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        <&Self as WorldQuery>::update_component_access(state, access);
    }

    fn init_state(world: &mut World) -> ComponentId {
        <&Self as WorldQuery>::init_state(world)
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        <&Self as WorldQuery>::get_state(components)
    }

    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        <&Self as WorldQuery>::matches_component_set(state, set_contains_id)
    }
}

#[allow(unsafe_code)]
/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl QueryData for Parent {
    type ReadOnly = Self;
}

#[allow(unsafe_code)]
/// SAFETY: access is read only
unsafe impl ReadOnlyQueryData for Parent {}
