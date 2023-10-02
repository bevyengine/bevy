use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{Component, ComponentId, ComponentStorage, StorageType, Tick},
    entity::Entity,
    query::{Access, DebugCheckedUnwrap, FilteredAccess, WorldQuery},
    storage::{Column, ComponentSparseSet, Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use bevy_utils::all_tuples;
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

// SAFETY: `Self::ReadOnly` is the same as `Self`
unsafe impl<T: Component> WorldQuery for With<T> {
    type Fetch<'w> = ();
    type Item<'w> = ();
    type ReadOnly = Self;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(_: Self::Item<'wlong>) -> Self::Item<'wshort> {}

    #[inline]
    unsafe fn init_fetch(
        _world: UnsafeWorldCell,
        _state: &ComponentId,
        _last_run: Tick,
        _this_run: Tick,
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
        _table: &Table,
    ) {
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }

    #[inline]
    fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
        access.and_with(id);
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

// SAFETY: `Self::ReadOnly` is the same as `Self`
unsafe impl<T: Component> WorldQuery for Without<T> {
    type Fetch<'w> = ();
    type Item<'w> = ();
    type ReadOnly = Self;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(_: Self::Item<'wlong>) -> Self::Item<'wshort> {}

    #[inline]
    unsafe fn init_fetch(
        _world: UnsafeWorldCell,
        _state: &ComponentId,
        _last_run: Tick,
        _this_run: Tick,
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
        _table: &Table,
    ) {
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }

    #[inline]
    fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
        access.and_without(id);
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
pub struct Or<T>(PhantomData<T>);

#[doc(hidden)]
pub struct OrFetch<'w, T: WorldQuery> {
    fetch: T::Fetch<'w>,
    matches: bool,
}

impl<T: WorldQuery> Clone for OrFetch<'_, T> {
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
        #[allow(clippy::unused_unit)]
        // SAFETY: defers to soundness of `$filter: WorldQuery` impl
        unsafe impl<$($filter: WorldQuery),*> WorldQuery for Or<($($filter,)*)> {
            type Fetch<'w> = ($(OrFetch<'w, $filter>,)*);
            type Item<'w> = bool;
            type ReadOnly = Or<($($filter::ReadOnly,)*)>;
            type State = ($($filter::State,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
                item
            }

            const IS_DENSE: bool = true $(&& $filter::IS_DENSE)*;

            const IS_ARCHETYPAL: bool = true $(&& $filter::IS_ARCHETYPAL)*;

            #[inline]
            unsafe fn init_fetch<'w>(world: UnsafeWorldCell<'w>, state: &Self::State, last_run: Tick, this_run: Tick) -> Self::Fetch<'w> {
                let ($($filter,)*) = state;
                ($(OrFetch {
                    fetch: $filter::init_fetch(world, $filter, last_run, this_run),
                    matches: false,
                },)*)
            }

            #[inline]
            unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table) {
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
            unsafe fn set_archetype<'w>(
                fetch: &mut Self::Fetch<'w>,
                state: & Self::State,
                archetype: &'w Archetype,
                table: &'w Table
            ) {
                let ($($filter,)*) = fetch;
                let ($($state,)*) = &state;
                $(
                    $filter.matches = $filter::matches_component_set($state, &|id| archetype.contains(id));
                    if $filter.matches {
                        $filter::set_archetype(&mut $filter.fetch, $state, archetype, table);
                    }
                )*
            }

            #[inline(always)]
            unsafe fn fetch<'w>(
                fetch: &mut Self::Fetch<'w>,
                _entity: Entity,
                _table_row: TableRow
            ) -> Self::Item<'w> {
                let ($($filter,)*) = fetch;
                false $(|| ($filter.matches && $filter::filter_fetch(&mut $filter.fetch, _entity, _table_row)))*
            }

            #[inline(always)]
            unsafe fn filter_fetch(
                fetch: &mut Self::Fetch<'_>,
                entity: Entity,
                table_row: TableRow
            ) -> bool {
                Self::fetch(fetch, entity, table_row)
            }

            fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
                let ($($filter,)*) = state;

                let mut _new_access = access.clone();
                let mut _not_first = false;
                $(
                    if _not_first {
                        let mut intermediate = access.clone();
                        $filter::update_component_access($filter, &mut intermediate);
                        _new_access.append_or(&intermediate);
                        _new_access.extend_access(&intermediate);
                    } else {
                        $filter::update_component_access($filter, &mut _new_access);
                        _not_first = true;
                    }
                )*

                *access = _new_access;
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
        $get_slice: expr,
        $get_sparse_set: expr
    ) => {
        $(#[$meta])*
        pub struct $name<T>(PhantomData<T>);

        #[doc(hidden)]
        #[derive(Clone)]
        $(#[$fetch_meta])*
        pub struct $fetch_name<'w> {
            table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<Tick>>>,
            sparse_set: Option<&'w ComponentSparseSet>,
            last_run: Tick,
            this_run: Tick,
        }

        // SAFETY: `Self::ReadOnly` is the same as `Self`
        unsafe impl<T: Component> WorldQuery for $name<T> {
            type Fetch<'w> = $fetch_name<'w>;
            type Item<'w> = bool;
            type ReadOnly = Self;
            type State = ComponentId;

            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
                item
            }

            #[inline]
            unsafe fn init_fetch<'w>(
                world: UnsafeWorldCell<'w>,
                &id: &ComponentId,
                last_run: Tick,
                this_run: Tick
            ) -> Self::Fetch<'w> {
                Self::Fetch::<'w> {
                    table_ticks: None,
                    sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet)
                        .then(|| {
                            world.storages()
                                 .sparse_sets
                                 .get(id)
                                 .debug_checked_unwrap()
                        }),
                    last_run,
                    this_run,
                }
            }

            const IS_DENSE: bool = {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => true,
                    StorageType::SparseSet => false,
                }
            };

            const IS_ARCHETYPAL:  bool = false;

            #[inline]
            unsafe fn set_table<'w>(
                fetch: &mut Self::Fetch<'w>,
                &component_id: &ComponentId,
                table: &'w Table
            ) {
                fetch.table_ticks = Some(
                    $get_slice(
                        &table
                            .get_column(component_id)
                            .debug_checked_unwrap()
                    ).into(),
                );
            }

            #[inline]
            unsafe fn set_archetype<'w>(
                fetch: &mut Self::Fetch<'w>,
                component_id: &ComponentId,
                _archetype: &'w Archetype,
                table: &'w Table
            ) {
                if Self::IS_DENSE {
                    Self::set_table(fetch, component_id, table);
                }
            }

            #[inline(always)]
            unsafe fn fetch<'w>(
                fetch: &mut Self::Fetch<'w>,
                entity: Entity,
                table_row: TableRow
            ) -> Self::Item<'w> {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => {
                        fetch
                            .table_ticks
                            .debug_checked_unwrap()
                            .get(table_row.index())
                            .deref()
                            .is_newer_than(fetch.last_run, fetch.this_run)
                    }
                    StorageType::SparseSet => {
                        let sparse_set = &fetch
                            .sparse_set
                            .debug_checked_unwrap();
                        $get_sparse_set(sparse_set, entity)
                            .debug_checked_unwrap()
                            .deref()
                            .is_newer_than(fetch.last_run, fetch.this_run)
                    }
                }
            }

            #[inline(always)]
            unsafe fn filter_fetch(
                fetch: &mut Self::Fetch<'_>,
                entity: Entity,
                table_row: TableRow
            ) -> bool {
                Self::fetch(fetch, entity, table_row)
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

        /// SAFETY: read-only access
        unsafe impl<T: Component> ReadOnlyWorldQuery for $name<T> {}
    };
}

impl_tick_filter!(
    /// A filter on a component that only retains results added after the system last ran.
    ///
    /// A common use for this filter is one-time initialization.
    ///
    /// To retain all results without filtering but still check whether they were added after the
    /// system last ran, use [`Ref<T>`](crate::change_detection::Ref).
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
    Column::get_added_ticks_slice,
    ComponentSparseSet::get_added_tick
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
    /// system last ran, use [`Ref<T>`](crate::change_detection::Ref).
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
    Column::get_changed_ticks_slice,
    ComponentSparseSet::get_changed_tick
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
