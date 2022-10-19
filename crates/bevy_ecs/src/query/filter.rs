use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{Component, ComponentId, ComponentStorage, ComponentTicks, StorageType},
    entity::Entity,
    query::{
        debug_checked_unreachable, Access, FilteredAccess, QueryFetch, WorldQuery, WorldQueryGats,
    },
    storage::{ComponentSparseSet, Table, Tables},
    world::World,
};
use bevy_ecs_macros::all_tuples;
use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use std::{cell::UnsafeCell, marker::PhantomData};

use super::ReadOnlyWorldQuery;

/// Filter that selects entities with a component `T`.
///
/// This can be used in a [`Query`](crate::system::Query) if entities are required to have the
/// component `T` but you don't actually care about components value.
///
/// This is the negation of [`Without`].
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::query::With;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component)]
/// # struct IsBeautiful;
/// # #[derive(Component)]
/// # struct Name { name: &'static str };
/// #
/// fn compliment_entity_system(query: Query<&Name, With<IsBeautiful>>) {
///     for name in &query {
///         println!("{} is looking lovely today!", name.name);
///     }
/// }
/// # bevy_ecs::system::assert_is_system(compliment_entity_system);
/// ```
pub struct With<T>(PhantomData<T>);

impl<T: Component> WorldQueryGats<'_> for With<T> {
    type Fetch = ();
    type Item = ();
}

// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for With<T> {
    type ReadOnly = Self;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(
        _: <Self as WorldQueryGats<'wlong>>::Item,
    ) -> <Self as WorldQueryGats<'wshort>>::Item {
    }

    unsafe fn init_fetch(
        _world: &World,
        _state: &ComponentId,
        _last_change_tick: u32,
        _change_tick: u32,
    ) {
    }

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    #[inline]
    unsafe fn set_table(_fetch: &mut (), _state: &ComponentId, _table: &Table) {}

    #[inline]
    unsafe fn set_archetype(
        _fetch: &mut (),
        _state: &ComponentId,
        _archetype: &Archetype,
        _tables: &Tables,
    ) {
    }

    #[inline]
    unsafe fn archetype_fetch<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        _archetype_index: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
    }

    #[inline]
    unsafe fn table_fetch<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        _table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
    }

    #[inline]
    fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
        access.add_with(id);
    }

    #[inline]
    fn update_archetype_component_access(
        _state: &ComponentId,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn matches_component_set(
        &id: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(id)
    }
}

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> ReadOnlyWorldQuery for With<T> {}

/// Filter that selects entities without a component `T`.
///
/// This is the negation of [`With`].
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::query::Without;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component)]
/// # struct Permit;
/// # #[derive(Component)]
/// # struct Name { name: &'static str };
/// #
/// fn no_permit_system(query: Query<&Name, Without<Permit>>) {
///     for name in &query{
///         println!("{} has no permit!", name.name);
///     }
/// }
/// # bevy_ecs::system::assert_is_system(no_permit_system);
/// ```
pub struct Without<T>(PhantomData<T>);

// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for Without<T> {
    type ReadOnly = Self;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(
        _: <Self as WorldQueryGats<'wlong>>::Item,
    ) -> <Self as WorldQueryGats<'wshort>>::Item {
    }

    unsafe fn init_fetch(
        _world: &World,
        _state: &ComponentId,
        _last_change_tick: u32,
        _change_tick: u32,
    ) {
    }

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    #[inline]
    unsafe fn set_table(_fetch: &mut (), _state: &Self::State, _table: &Table) {}

    #[inline]
    unsafe fn set_archetype(
        _fetch: &mut (),
        _state: &ComponentId,
        _archetype: &Archetype,
        _tables: &Tables,
    ) {
    }

    #[inline]
    unsafe fn archetype_fetch<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        _archetype_index: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
    }

    #[inline]
    unsafe fn table_fetch<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        _table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
    }

    #[inline]
    fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
        access.add_without(id);
    }

    #[inline]
    fn update_archetype_component_access(
        _state: &ComponentId,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn matches_component_set(
        &id: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        !set_contains_id(id)
    }
}

impl<T: Component> WorldQueryGats<'_> for Without<T> {
    type Fetch = ();
    type Item = ();
}

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> ReadOnlyWorldQuery for Without<T> {}

/// A filter that tests if any of the given filters apply.
///
/// This is useful for example if a system with multiple components in a query only wants to run
/// when one or more of the components have changed.
///
/// The `And` equivalent to this filter is a [`prim@tuple`] testing that all the contained filters
/// apply instead.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// # use bevy_ecs::query::Changed;
/// # use bevy_ecs::query::Or;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component, Debug)]
/// # struct Color {};
/// # #[derive(Component)]
/// # struct Style {};
/// #
/// fn print_cool_entity_system(query: Query<Entity, Or<(Changed<Color>, Changed<Style>)>>) {
///     for entity in &query {
///         println!("Entity {:?} got a new style or color", entity);
///     }
/// }
/// # bevy_ecs::system::assert_is_system(print_cool_entity_system);
/// ```
#[derive(Clone, Copy)]
pub struct Or<T>(pub T);

#[doc(hidden)]
pub struct OrFetch<'w, T: WorldQuery> {
    fetch: QueryFetch<'w, T>,
    matches: bool,
}
impl<'w, T: WorldQuery> Copy for OrFetch<'w, T> where QueryFetch<'w, T>: Copy {}
impl<'w, T: WorldQuery> Clone for OrFetch<'w, T>
where
    QueryFetch<'w, T>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            fetch: self.fetch.clone(),
            matches: self.matches,
        }
    }
}

macro_rules! impl_query_filter_tuple {
    ($(($filter: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, $($filter: WorldQuery),*> WorldQueryGats<'w> for Or<($($filter,)*)> {
            type Fetch = ($(OrFetch<'w, $filter>,)*);
            type Item = bool;
        }


        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        // SAFETY: defers to soundness of `$filter: WorldQuery` impl
        unsafe impl<$($filter: WorldQuery),*> WorldQuery for Or<($($filter,)*)> {
            type ReadOnly = Or<($($filter::ReadOnly,)*)>;
            type State = ($($filter::State,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: super::QueryItem<'wlong, Self>) -> super::QueryItem<'wshort, Self> {
                item
            }

            const IS_DENSE: bool = true $(&& $filter::IS_DENSE)*;

            const IS_ARCHETYPAL: bool = true $(&& $filter::IS_ARCHETYPAL)*;

            unsafe fn init_fetch<'w>(world: &'w World, state: &Self::State, last_change_tick: u32, change_tick: u32) -> <Self as WorldQueryGats<'w>>::Fetch {
                let ($($filter,)*) = state;
                ($(OrFetch {
                    fetch: $filter::init_fetch(world, $filter, last_change_tick, change_tick),
                    matches: false,
                },)*)
            }

            #[inline]
            unsafe fn set_table<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, state: &Self::State, table: &'w Table) {
                let ($($filter,)*) = fetch;
                let ($($state,)*) = state;
                $(
                    $filter.matches = $filter::matches_component_set($state, &|id| table.has_column(id));
                    if $filter.matches {
                        $filter::set_table(&mut $filter.fetch, $state, table);
                    }
                )*
            }

            #[inline]
            unsafe fn set_archetype<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, state: &Self::State, archetype: &'w Archetype, tables: &'w Tables) {
                let ($($filter,)*) = fetch;
                let ($($state,)*) = state;
                $(
                    $filter.matches = $filter::matches_component_set($state, &|id| archetype.contains(id));
                    if $filter.matches {
                        $filter::set_archetype(&mut $filter.fetch, $state, archetype, tables);
                    }
                )*
            }

            #[inline]
            unsafe fn table_fetch<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, table_row: usize) -> <Self as WorldQueryGats<'w>>::Item {
                let ($($filter,)*) = fetch;
                false $(|| ($filter.matches && $filter::table_filter_fetch(&mut $filter.fetch, table_row)))*
            }

            #[inline]
            unsafe fn archetype_fetch<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, archetype_index: usize) -> <Self as WorldQueryGats<'w>>::Item {
                let ($($filter,)*) = fetch;
                false $(|| ($filter.matches && $filter::archetype_filter_fetch(&mut $filter.fetch, archetype_index)))*
            }

            #[inline]
            unsafe fn table_filter_fetch(fetch: &mut QueryFetch<'_, Self>, table_row: usize) -> bool {
                Self::table_fetch(fetch, table_row)
            }

            #[inline]
            unsafe fn archetype_filter_fetch(fetch: &mut QueryFetch<'_, Self>, archetype_index: usize) -> bool {
                Self::archetype_fetch(fetch, archetype_index)
            }

            fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
                let ($($filter,)*) = state;

                // We do not unconditionally add `$filter`'s `with`/`without` accesses to `access`
                // as this would be unsound. For example the following two queries should conflict:
                // - Query<&mut B, Or<(With<A>, ())>>
                // - Query<&mut B, Without<A>>
                //
                // If we were to unconditionally add `$name`'s `with`/`without` accesses then `Or<(With<A>, ())>`
                // would have a `With<A>` access which is incorrect as this `WorldQuery` will match entities that
                // do not have the `A` component. This is the same logic as the `AnyOf<...>: WorldQuery` impl.
                //
                // The correct thing to do here is to only add a `with`/`without` access to `_access` if all
                // `$filter` params have that `with`/`without` access. More jargony put- we add the intersection
                // of all `with`/`without` accesses of the `$filter` params to `access`.
                let mut _intersected_access = access.clone();
                let mut _not_first = false;
                $(
                    if _not_first {
                        let mut intermediate = access.clone();
                        $filter::update_component_access($filter, &mut intermediate);
                        _intersected_access.extend_intersect_filter(&intermediate);
                        _intersected_access.extend_access(&intermediate);
                    } else {
                        $filter::update_component_access($filter, &mut _intersected_access);
                        _not_first = true;
                    }
                )*

                *access = _intersected_access;
            }

            fn update_archetype_component_access(state: &Self::State, archetype: &Archetype, access: &mut Access<ArchetypeComponentId>) {
                let ($($filter,)*) = state;
                $($filter::update_archetype_component_access($filter, archetype, access);)*
            }

            fn init_state(world: &mut World) -> Self::State {
                ($($filter::init_state(world),)*)
            }

            fn matches_component_set(_state: &Self::State, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($filter,)*) = _state;
                false $(|| $filter::matches_component_set($filter, _set_contains_id))*
            }
        }

        // SAFETY: filters are read only
        unsafe impl<$($filter: ReadOnlyWorldQuery),*> ReadOnlyWorldQuery for Or<($($filter,)*)> {}
    };
}

all_tuples!(impl_query_filter_tuple, 0, 15, F, S);

macro_rules! impl_tick_filter {
    (
        $(#[$meta:meta])*
        $name: ident,
        $(#[$fetch_meta:meta])*
        $fetch_name: ident,
        $is_detected: expr
    ) => {
        $(#[$meta])*
        pub struct $name<T>(PhantomData<T>);

        #[doc(hidden)]
        $(#[$fetch_meta])*
        pub struct $fetch_name<'w, T> {
            table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>>,
            entity_table_rows: Option<ThinSlicePtr<'w, usize>>,
            marker: PhantomData<T>,
            entities: Option<ThinSlicePtr<'w, Entity>>,
            sparse_set: Option<&'w ComponentSparseSet>,
            last_change_tick: u32,
            change_tick: u32,
        }

        // SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
        unsafe impl<T: Component> WorldQuery for $name<T> {
            type ReadOnly = Self;
            type State = ComponentId;

            fn shrink<'wlong: 'wshort, 'wshort>(item: super::QueryItem<'wlong, Self>) -> super::QueryItem<'wshort, Self> {
                item
            }

            unsafe fn init_fetch<'w>(world: &'w World, &id: &ComponentId, last_change_tick: u32, change_tick: u32) -> <Self as WorldQueryGats<'w>>::Fetch {
                QueryFetch::<'w, Self> {
                    table_ticks: None,
                    entities: None,
                    entity_table_rows: None,
                    sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet)
                        .then(|| world.storages().sparse_sets.get(id).unwrap()),
                    marker: PhantomData,
                    last_change_tick,
                    change_tick,
                }
            }

            const IS_DENSE: bool = {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => true,
                    StorageType::SparseSet => false,
                }
            };

            const IS_ARCHETYPAL:  bool = false;

            unsafe fn set_table<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, &id: &ComponentId, table: &'w Table) {
                fetch.table_ticks = Some(table.get_column(id).unwrap().get_ticks_slice().into());
            }

            unsafe fn set_archetype<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, &id: &ComponentId, archetype: &'w Archetype, tables: &'w Tables) {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => {
                        fetch.entity_table_rows = Some(archetype.entity_table_rows().into());
                        let table = &tables[archetype.table_id()];
                        fetch.table_ticks = Some(table.get_column(id).unwrap().get_ticks_slice().into());
                    }
                    StorageType::SparseSet => fetch.entities = Some(archetype.entities().into()),
                }
            }

            unsafe fn table_fetch<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, table_row: usize) -> <Self as WorldQueryGats<'w>>::Item {
                $is_detected(&*(fetch.table_ticks.unwrap_or_else(|| debug_checked_unreachable()).get(table_row)).deref(), fetch.last_change_tick, fetch.change_tick)
            }

            unsafe fn archetype_fetch<'w>(fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, archetype_index: usize) -> <Self as WorldQueryGats<'w>>::Item {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => {
                        let table_row = *fetch.entity_table_rows.unwrap_or_else(|| debug_checked_unreachable()).get(archetype_index);
                        $is_detected(&*(fetch.table_ticks.unwrap_or_else(|| debug_checked_unreachable()).get(table_row)).deref(), fetch.last_change_tick, fetch.change_tick)
                    }
                    StorageType::SparseSet => {
                        let entity = *fetch.entities.unwrap_or_else(|| debug_checked_unreachable()).get(archetype_index);
                        let ticks = fetch
                            .sparse_set
                            .unwrap_or_else(|| debug_checked_unreachable())
                            .get_ticks(entity)
                            .map(|ticks| &*ticks.get())
                            .cloned()
                            .unwrap();
                        $is_detected(&ticks, fetch.last_change_tick, fetch.change_tick)
                    }
                }
            }

            #[inline]
            unsafe fn table_filter_fetch(fetch: &mut QueryFetch<'_, Self>, table_row: usize) -> bool {
                Self::table_fetch(fetch, table_row)
            }

            #[inline]
            unsafe fn archetype_filter_fetch(fetch: &mut QueryFetch<'_, Self>, archetype_index: usize) -> bool {
                Self::archetype_fetch(fetch, archetype_index)
            }

            #[inline]
            fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
                if access.access().has_write(id) {
                    panic!("$state_name<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                        std::any::type_name::<T>());
                }
                access.add_read(id);
            }

            #[inline]
            fn update_archetype_component_access(
                &id: &ComponentId,
                archetype: &Archetype,
                access: &mut Access<ArchetypeComponentId>,
            ) {
                if let Some(archetype_component_id) = archetype.get_archetype_component_id(id) {
                    access.add_read(archetype_component_id);
                }
            }

            fn init_state(world: &mut World) -> ComponentId {
                world.init_component::<T>()
            }

            fn matches_component_set(&id: &ComponentId, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                set_contains_id(id)
            }
        }

        impl<'w, T: Component> WorldQueryGats<'w> for $name<T> {
            type Fetch = $fetch_name<'w, T>;
            type Item = bool;
        }

        /// SAFETY: read-only access
        unsafe impl<T: Component> ReadOnlyWorldQuery for $name<T> {}

        impl<T> Clone for $fetch_name<'_, T> {
            fn clone(&self) -> Self {
                Self {
                    table_ticks: self.table_ticks.clone(),
                    entity_table_rows: self.entity_table_rows.clone(),
                    marker: self.marker.clone(),
                    entities: self.entities.clone(),
                    sparse_set: self.sparse_set.clone(),
                    last_change_tick: self.last_change_tick.clone(),
                    change_tick: self.change_tick.clone(),
                }
            }
        }

        impl<T> Copy for $fetch_name<'_, T> {}
    };
}

impl_tick_filter!(
    /// A filter on a component that only retains results added after the system last ran.
    ///
    /// A common use for this filter is one-time initialization.
    ///
    /// To retain all results without filtering but still check whether they were added after the
    /// system last ran, use [`ChangeTrackers<T>`](crate::query::ChangeTrackers).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::component::Component;
    /// # use bevy_ecs::query::Added;
    /// # use bevy_ecs::system::IntoSystem;
    /// # use bevy_ecs::system::Query;
    /// #
    /// # #[derive(Component, Debug)]
    /// # struct Name {};
    ///
    /// fn print_add_name_component(query: Query<&Name, Added<Name>>) {
    ///     for name in &query {
    ///         println!("Named entity created: {:?}", name)
    ///     }
    /// }
    ///
    /// # bevy_ecs::system::assert_is_system(print_add_name_component);
    /// ```
    Added,
    AddedFetch,
    ComponentTicks::is_added
);

impl_tick_filter!(
    /// A filter on a component that only retains results added or mutably dereferenced after the system last ran.
    ///  
    /// A common use for this filter is avoiding redundant work when values have not changed.
    ///
    /// **Note** that simply *mutably dereferencing* a component is considered a change ([`DerefMut`](std::ops::DerefMut)).
    /// Bevy does not compare components to their previous values.
    ///
    /// To retain all results without filtering but still check whether they were changed after the
    /// system last ran, use [`ChangeTrackers<T>`](crate::query::ChangeTrackers).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::component::Component;
    /// # use bevy_ecs::query::Changed;
    /// # use bevy_ecs::system::IntoSystem;
    /// # use bevy_ecs::system::Query;
    /// #
    /// # #[derive(Component, Debug)]
    /// # struct Name {};
    /// # #[derive(Component)]
    /// # struct Transform {};
    ///
    /// fn print_moving_objects_system(query: Query<&Name, Changed<Transform>>) {
    ///     for name in &query {
    ///         println!("Entity Moved: {:?}", name);
    ///     }
    /// }
    ///
    /// # bevy_ecs::system::assert_is_system(print_moving_objects_system);
    /// ```
    Changed,
    ChangedFetch,
    ComponentTicks::is_changed
);

/// A marker trait to indicate that the filter works at an archetype level.
///
/// This is needed to implement [`ExactSizeIterator`](std::iter::ExactSizeIterator) for
/// [`QueryIter`](crate::query::QueryIter) that contains archetype-level filters.
///
/// The trait must only be implement for filters where its corresponding [`WorldQuery::IS_ARCHETYPAL`](crate::query::WorldQuery::IS_ARCHETYPAL)
/// is [`prim@true`]. As such, only the [`With`] and [`Without`] filters can implement the trait.
/// [Tuples](prim@tuple) and [`Or`] filters are automatically implemented with the trait only if its containing types
/// also implement the same trait.
///
/// [`Added`] and [`Changed`] works with entities, and therefore are not archetypal. As such
/// they do not implement [`ArchetypeFilter`].
pub trait ArchetypeFilter {}

impl<T> ArchetypeFilter for With<T> {}
impl<T> ArchetypeFilter for Without<T> {}

macro_rules! impl_archetype_filter_tuple {
    ($($filter: ident),*) => {
        impl<$($filter: ArchetypeFilter),*> ArchetypeFilter for ($($filter,)*) {}

        impl<$($filter: ArchetypeFilter),*> ArchetypeFilter for Or<($($filter,)*)> {}
    };
}

all_tuples!(impl_archetype_filter_tuple, 0, 15, F);
