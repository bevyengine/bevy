#[cfg(feature = "reflect")]
use bevy_ecs::reflect::{ReflectComponent, ReflectMapEntities};
use bevy_ecs::{
    archetype::Archetype,
    component::{Component, ComponentId, Components, Tick},
    entity::{Entity, EntityMapper, MapEntities},
    prelude::FromWorld,
    query::{FilteredAccess, QueryData, ReadFetch, ReadOnlyQueryData, WorldQuery},
    storage::{Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use core::slice;
use smallvec::SmallVec;
use std::ops::Deref;

/// Contains references to the child entities of this entity.
///
/// Each child must contain a [`Parent`] component that points back to this entity.
/// This component rarely needs to be created manually,
/// consider using higher level utilities like [`BuildChildren::with_children`]
/// which are safer and easier to use.
///
/// See [`HierarchyQueryExt`] for hierarchy related methods on [`Query`].
///
/// [`HierarchyQueryExt`]: crate::query_extension::HierarchyQueryExt
/// [`Query`]: bevy_ecs::system::Query
/// [`Parent`]: crate::components::parent::Parent
/// [`BuildChildren::with_children`]: crate::child_builder::BuildChildren::with_children
#[derive(Component, Debug)]
#[cfg_attr(feature = "reflect", derive(bevy_reflect::Reflect))]
#[cfg_attr(feature = "reflect", reflect(Component, MapEntities))]
pub struct Children(pub(crate) SmallVec<[Entity; 8]>);

impl MapEntities for Children {
    fn map_entities<M: EntityMapper>(&mut self, entity_mapper: &mut M) {
        for entity in &mut self.0 {
            *entity = entity_mapper.map_entity(*entity);
        }
    }
}

// TODO: We need to impl either FromWorld or Default so Children can be registered as Reflect.
// This is because Reflect deserialize by creating an instance and apply a patch on top.
// However Children should only ever be set with a real user-defined entities. Its worth looking
// into better ways to handle cases like this.
impl FromWorld for Children {
    #[inline]
    fn from_world(_world: &mut World) -> Self {
        Children(SmallVec::new())
    }
}

impl Children {
    /// Constructs a [`Children`] component with the given entities.
    #[inline]
    pub(crate) fn from_entities(entities: &[Entity]) -> Self {
        Self(SmallVec::from_slice(entities))
    }

    /// Swaps the child at `a_index` with the child at `b_index`.
    #[inline]
    pub fn swap(&mut self, a_index: usize, b_index: usize) {
        self.0.swap(a_index, b_index);
    }

    /// Sorts children [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_by`].
    ///
    /// For the unstable version, see [`sort_unstable_by`](Children::sort_unstable_by).
    ///
    /// See also [`sort_by_key`](Children::sort_by_key), [`sort_by_cached_key`](Children::sort_by_cached_key).
    #[inline]
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> std::cmp::Ordering,
    {
        self.0.sort_by(compare);
    }

    /// Sorts children [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_by_key`].
    ///
    /// For the unstable version, see [`sort_unstable_by_key`](Children::sort_unstable_by_key).
    ///
    /// See also [`sort_by`](Children::sort_by), [`sort_by_cached_key`](Children::sort_by_cached_key).
    #[inline]
    pub fn sort_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.0.sort_by_key(compare);
    }

    /// Sorts children [stably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function. Only evaluates each key at most
    /// once per sort, caching the intermediate results in memory.
    ///
    /// For the underlying implementation, see [`slice::sort_by_cached_key`].
    ///
    /// See also [`sort_by`](Children::sort_by), [`sort_by_key`](Children::sort_by_key).
    #[inline]
    pub fn sort_by_cached_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.0.sort_by_cached_key(compare);
    }

    /// Sorts children [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided comparator function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by`].
    ///
    /// For the stable version, see [`sort_by`](Children::sort_by).
    ///
    /// See also [`sort_unstable_by_key`](Children::sort_unstable_by_key).
    #[inline]
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: FnMut(&Entity, &Entity) -> std::cmp::Ordering,
    {
        self.0.sort_unstable_by(compare);
    }

    /// Sorts children [unstably](https://en.wikipedia.org/wiki/Sorting_algorithm#Stability)
    /// in place using the provided key extraction function.
    ///
    /// For the underlying implementation, see [`slice::sort_unstable_by_key`].
    ///
    /// For the stable version, see [`sort_by_key`](Children::sort_by_key).
    ///
    /// See also [`sort_unstable_by`](Children::sort_unstable_by).
    #[inline]
    pub fn sort_unstable_by_key<K, F>(&mut self, compare: F)
    where
        F: FnMut(&Entity) -> K,
        K: Ord,
    {
        self.0.sort_unstable_by_key(compare);
    }
}

impl Deref for Children {
    type Target = [Entity];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl<'a> IntoIterator for &'a Children {
    type Item = <Self::IntoIter as Iterator>::Item;

    type IntoIter = slice::Iter<'a, Entity>;

    #[inline(always)]
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[allow(unsafe_code)]
/// SAFETY:
/// This implementation delegates to the existing implementation for &Children
unsafe impl WorldQuery for Children
where
    Self: Component,
{
    type Item<'w> = &'w [Entity];
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
        unsafe {
            <&Self as WorldQuery>::fetch(fetch, entity, table_row)
                .0
                .as_ref()
        }
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
unsafe impl QueryData for Children {
    type ReadOnly = Self;
}

#[allow(unsafe_code)]
/// SAFETY: access is read only
unsafe impl ReadOnlyQueryData for Children {}
