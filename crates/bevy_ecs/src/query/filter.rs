use crate::{
    archetype::Archetype,
    component::{Component, ComponentId, Components, StorageType, Tick},
    entity::Entity,
    query::{DebugCheckedUnwrap, FilteredAccess, WorldQuery},
    storage::{Column, ComponentSparseSet, Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use bevy_utils::all_tuples;
use std::{cell::UnsafeCell, marker::PhantomData};

/// Types that filter the results of a [`Query`].
///
/// There are many types that natively implement this trait:
/// - **Component filters.**
///   [`With`] and [`Without`] filters can be applied to check if the queried entity does or does not contain a particular component.
/// - **Change detection filters.**
///   [`Added`] and [`Changed`] filters can be applied to detect component changes to an entity.
/// - **`QueryFilter` tuples.**
///   If every element of a tuple implements `QueryFilter`, then the tuple itself also implements the same trait.
///   This enables a single `Query` to filter over multiple conditions.
///   Due to the current lack of variadic generics in Rust, the trait has been implemented for tuples from 0 to 15 elements,
///   but nesting of tuples allows infinite `QueryFilter`s.
/// - **Filter disjunction operator.**
///   By default, tuples compose query filters in such a way that all conditions must be satisfied to generate a query item for a given entity.
///   Wrapping a tuple inside an [`Or`] operator will relax the requirement to just one condition.
///
/// Implementing the trait manually can allow for a fundamentally new type of behavior.
///
/// Query design can be easily structured by deriving `QueryFilter` for custom types.
/// Despite the added complexity, this approach has several advantages over using `QueryFilter` tuples.
/// The most relevant improvements are:
///
/// - Reusability across multiple systems.
/// - Filters can be composed together to create a more complex filter.
///
/// This trait can only be derived for structs if each field also implements `QueryFilter`.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::{query::QueryFilter, component::Component};
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # #[derive(Component)]
/// # struct ComponentC;
/// # #[derive(Component)]
/// # struct ComponentD;
/// # #[derive(Component)]
/// # struct ComponentE;
/// #
/// #[derive(QueryFilter)]
/// struct MyFilter<T: Component, P: Component> {
///     // Field names are not relevant, since they are never manually accessed.
///     with_a: With<ComponentA>,
///     or_filter: Or<(With<ComponentC>, Added<ComponentB>)>,
///     generic_tuple: (With<T>, Without<P>),
/// }
///
/// fn my_system(query: Query<Entity, MyFilter<ComponentD, ComponentE>>) {
///     // ...
/// }
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// [`fetch`]: Self::fetch
/// [`matches_component_set`]: Self::matches_component_set
/// [`Query`]: crate::system::Query
/// [`State`]: Self::State
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `Query` filter",
    label = "invalid `Query` filter",
    note = "a `QueryFilter` typically uses a combination of `With<T>` and `Without<T>` statements"
)]
pub trait QueryFilter: WorldQuery {
    /// Returns true if (and only if) this Filter relies strictly on archetypes to limit which
    /// components are accessed by the Query.
    ///
    /// This enables optimizations for [`crate::query::QueryIter`] that rely on knowing exactly how
    /// many elements are being iterated (such as `Iterator::collect()`).
    const IS_ARCHETYPAL: bool;

    /// Returns true if the provided [`Entity`] and [`TableRow`] should be included in the query results.
    /// If false, the entity will be skipped.
    ///
    /// Note that this is called after already restricting the matched [`Table`]s and [`Archetype`]s to the
    /// ones that are compatible with the Filter's access.
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`WorldQuery::set_table`] or [`WorldQuery::set_archetype`]. `entity` and
    /// `table_row` must be in the range of the current table and archetype.
    #[allow(unused_variables)]
    unsafe fn filter_fetch(
        fetch: &mut Self::Fetch<'_>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool;
}

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

/// SAFETY:
/// `update_component_access` does not add any accesses.
/// This is sound because `fetch` does not access any components.
/// `update_component_access` adds a `With` filter for `T`.
/// This is sound because `matches_component_set` returns whether the set contains the component.
unsafe impl<T: Component> WorldQuery for With<T> {
    type Item<'w> = ();
    type Fetch<'w> = ();
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
        match T::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    #[inline]
    unsafe fn set_archetype(
        _fetch: &mut (),
        _state: &ComponentId,
        _archetype: &Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table(_fetch: &mut (), _state: &ComponentId, _table: &Table) {}

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

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        &id: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(id)
    }
}

impl<T: Component> QueryFilter for With<T> {
    const IS_ARCHETYPAL: bool = true;

    #[inline(always)]
    unsafe fn filter_fetch(
        _fetch: &mut Self::Fetch<'_>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> bool {
        true
    }
}

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

/// SAFETY:
/// `update_component_access` does not add any accesses.
/// This is sound because `fetch` does not access any components.
/// `update_component_access` adds a `Without` filter for `T`.
/// This is sound because `matches_component_set` returns whether the set does not contain the component.
unsafe impl<T: Component> WorldQuery for Without<T> {
    type Item<'w> = ();
    type Fetch<'w> = ();
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
        match T::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    #[inline]
    unsafe fn set_archetype(
        _fetch: &mut (),
        _state: &ComponentId,
        _archetype: &Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table(_fetch: &mut (), _state: &Self::State, _table: &Table) {}

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

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        &id: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        !set_contains_id(id)
    }
}

impl<T: Component> QueryFilter for Without<T> {
    const IS_ARCHETYPAL: bool = true;

    #[inline(always)]
    unsafe fn filter_fetch(
        _fetch: &mut Self::Fetch<'_>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> bool {
        true
    }
}

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

macro_rules! impl_or_query_filter {
    ($(($filter: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        /// SAFETY:
        /// `fetch` accesses are a subset of the subqueries' accesses
        /// This is sound because `update_component_access` adds accesses according to the implementations of all the subqueries.
        /// `update_component_access` replace the filters with a disjunction where every element is a conjunction of the previous filters and the filters of one of the subqueries.
        /// This is sound because `matches_component_set` returns a disjunction of the results of the subqueries' implementations.
        unsafe impl<$($filter: QueryFilter),*> WorldQuery for Or<($($filter,)*)> {
            type Fetch<'w> = ($(OrFetch<'w, $filter>,)*);
            type Item<'w> = bool;
            type State = ($($filter::State,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
                item
            }

            const IS_DENSE: bool = true $(&& $filter::IS_DENSE)*;

            #[inline]
            unsafe fn init_fetch<'w>(world: UnsafeWorldCell<'w>, state: &Self::State, last_run: Tick, this_run: Tick) -> Self::Fetch<'w> {
                let ($($filter,)*) = state;
                ($(OrFetch {
                    // SAFETY: The invariants are uphold by the caller.
                    fetch: unsafe { $filter::init_fetch(world, $filter, last_run, this_run) },
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
                        // SAFETY: The invariants are uphold by the caller.
                        unsafe { $filter::set_table(&mut $filter.fetch, $state, table); }
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
                        // SAFETY: The invariants are uphold by the caller.
                       unsafe { $filter::set_archetype(&mut $filter.fetch, $state, archetype, table); }
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
                // SAFETY: The invariants are uphold by the caller.
                false $(|| ($filter.matches && unsafe { $filter::filter_fetch(&mut $filter.fetch, _entity, _table_row) }))*
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
                        _new_access.required = access.required.clone();
                        _not_first = true;
                    }
                )*

                *access = _new_access;
            }

            fn init_state(world: &mut World) -> Self::State {
                ($($filter::init_state(world),)*)
            }

            fn get_state(components: &Components) -> Option<Self::State> {
                Some(($($filter::get_state(components)?,)*))
            }

            fn matches_component_set(_state: &Self::State, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($filter,)*) = _state;
                false $(|| $filter::matches_component_set($filter, _set_contains_id))*
            }
        }

        impl<$($filter: QueryFilter),*> QueryFilter for Or<($($filter,)*)> {
            const IS_ARCHETYPAL: bool = true $(&& $filter::IS_ARCHETYPAL)*;

            #[inline(always)]
            unsafe fn filter_fetch(
                fetch: &mut Self::Fetch<'_>,
                entity: Entity,
                table_row: TableRow
            ) -> bool {
                // SAFETY: The invariants are uphold by the caller.
                unsafe { Self::fetch(fetch, entity, table_row) }
            }
        }
    };
}

macro_rules! impl_tuple_query_filter {
    ($($name: ident),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]

        impl<$($name: QueryFilter),*> QueryFilter for ($($name,)*) {
            const IS_ARCHETYPAL: bool = true $(&& $name::IS_ARCHETYPAL)*;

            #[inline(always)]
            unsafe fn filter_fetch(
                fetch: &mut Self::Fetch<'_>,
                _entity: Entity,
                _table_row: TableRow
            ) -> bool {
                let ($($name,)*) = fetch;
                // SAFETY: The invariants are uphold by the caller.
                true $(&& unsafe { $name::filter_fetch($name, _entity, _table_row) })*
            }
        }

    };
}

all_tuples!(impl_tuple_query_filter, 0, 15, F);
all_tuples!(impl_or_query_filter, 0, 15, F, S);

/// A filter on a component that only retains results the first time after they have been added.
///
/// A common use for this filter is one-time initialization.
///
/// To retain all results without filtering but still check whether they were added after the
/// system last ran, use [`Ref<T>`](crate::change_detection::Ref).
///
/// **Note** that this includes changes that happened before the first time this `Query` was run.
///
/// # Deferred
///
/// Note, that entity modifications issued with [`Commands`](crate::system::Commands)
/// are visible only after deferred operations are applied,
/// typically at the end of the schedule iteration.
///
/// # Time complexity
///
/// `Added` is not [`ArchetypeFilter`], which practically means that
/// if the query (with `T` component filter) matches a million entities,
/// `Added<T>` filter will iterate over all of them even if none of them were just added.
///
/// For example, these two systems are roughly equivalent in terms of performance:
///
/// ```
/// # use bevy_ecs::change_detection::{DetectChanges, Ref};
/// # use bevy_ecs::entity::Entity;
/// # use bevy_ecs::query::Added;
/// # use bevy_ecs::system::Query;
/// # use bevy_ecs_macros::Component;
/// # #[derive(Component)]
/// # struct MyComponent;
/// # #[derive(Component)]
/// # struct Transform;
///
/// fn system1(q: Query<&MyComponent, Added<Transform>>) {
///     for item in &q { /* component added */ }
/// }
///
/// fn system2(q: Query<(&MyComponent, Ref<Transform>)>) {
///     for item in &q {
///         if item.1.is_added() { /* component added */ }
///     }
/// }
/// ```
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
pub struct Added<T>(PhantomData<T>);

#[doc(hidden)]
#[derive(Clone)]
pub struct AddedFetch<'w> {
    table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<Tick>>>,
    sparse_set: Option<&'w ComponentSparseSet>,
    last_run: Tick,
    this_run: Tick,
}

/// SAFETY:
/// `fetch` accesses a single component in a readonly way.
/// This is sound because `update_component_access` adds read access for that component and panics when appropriate.
/// `update_component_access` adds a `With` filter for a component.
/// This is sound because `matches_component_set` returns whether the set contains that component.
unsafe impl<T: Component> WorldQuery for Added<T> {
    type Item<'w> = bool;
    type Fetch<'w> = AddedFetch<'w>;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        &id: &ComponentId,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w> {
        Self::Fetch::<'w> {
            table_ticks: None,
            sparse_set: (T::STORAGE_TYPE == StorageType::SparseSet)
                .then(|| world.storages().sparse_sets.get(id).debug_checked_unwrap()),
            last_run,
            this_run,
        }
    }

    const IS_DENSE: bool = {
        match T::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        component_id: &ComponentId,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if Self::IS_DENSE {
            // SAFETY: `set_archetype`'s safety rules are a super set of the `set_table`'s ones.
            unsafe {
                Self::set_table(fetch, component_id, table);
            }
        }
    }

    #[inline]
    unsafe fn set_table<'w>(
        fetch: &mut Self::Fetch<'w>,
        &component_id: &ComponentId,
        table: &'w Table,
    ) {
        fetch.table_ticks = Some(
            Column::get_added_ticks_slice(table.get_column(component_id).debug_checked_unwrap())
                .into(),
        );
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        match T::STORAGE_TYPE {
            StorageType::Table => {
                // SAFETY: STORAGE_TYPE = Table
                let table = unsafe { fetch.table_ticks.debug_checked_unwrap() };
                // SAFETY: The caller ensures `table_row` is in range.
                let tick = unsafe { table.get(table_row.as_usize()) };

                tick.deref().is_newer_than(fetch.last_run, fetch.this_run)
            }
            StorageType::SparseSet => {
                // SAFETY: STORAGE_TYPE = SparseSet
                let sparse_set = unsafe { &fetch.sparse_set.debug_checked_unwrap() };
                // SAFETY: The caller ensures `entity` is in range.
                let tick = unsafe {
                    ComponentSparseSet::get_added_tick(sparse_set, entity).debug_checked_unwrap()
                };

                tick.deref().is_newer_than(fetch.last_run, fetch.this_run)
            }
        }
    }

    #[inline]
    fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
        if access.access().has_write(id) {
            panic!("$state_name<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",std::any::type_name::<T>());
        }
        access.add_read(id);
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn get_state(components: &Components) -> Option<ComponentId> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        &id: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(id)
    }
}

impl<T: Component> QueryFilter for Added<T> {
    const IS_ARCHETYPAL: bool = false;
    #[inline(always)]
    unsafe fn filter_fetch(
        fetch: &mut Self::Fetch<'_>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        // SAFETY: The invariants are uphold by the caller.
        unsafe { Self::fetch(fetch, entity, table_row) }
    }
}

/// A filter on a component that only retains results the first time after they have been added or mutably dereferenced.
///
/// A common use for this filter is avoiding redundant work when values have not changed.
///
/// **Note** that simply *mutably dereferencing* a component is considered a change ([`DerefMut`](std::ops::DerefMut)).
/// Bevy does not compare components to their previous values.
///
/// To retain all results without filtering but still check whether they were changed after the
/// system last ran, use [`Ref<T>`](crate::change_detection::Ref).
///
/// **Note** that this includes changes that happened before the first time this `Query` was run.
///
/// # Deferred
///
/// Note, that entity modifications issued with [`Commands`](crate::system::Commands)
/// (like entity creation or entity component addition or removal)
/// are visible only after deferred operations are applied,
/// typically at the end of the schedule iteration.
///
/// # Time complexity
///
/// `Changed` is not [`ArchetypeFilter`], which practically means that
/// if query (with `T` component filter) matches million entities,
/// `Changed<T>` filter will iterate over all of them even if none of them were changed.
///
/// For example, these two systems are roughly equivalent in terms of performance:
///
/// ```
/// # use bevy_ecs::change_detection::DetectChanges;
/// # use bevy_ecs::entity::Entity;
/// # use bevy_ecs::query::Changed;
/// # use bevy_ecs::system::Query;
/// # use bevy_ecs::world::Ref;
/// # use bevy_ecs_macros::Component;
/// # #[derive(Component)]
/// # struct MyComponent;
/// # #[derive(Component)]
/// # struct Transform;
///
/// fn system1(q: Query<&MyComponent, Changed<Transform>>) {
///     for item in &q { /* component changed */ }
/// }
///
/// fn system2(q: Query<(&MyComponent, Ref<Transform>)>) {
///     for item in &q {
///         if item.1.is_changed() { /* component changed */ }
///     }
/// }
/// ```
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
pub struct Changed<T>(PhantomData<T>);

#[doc(hidden)]
#[derive(Clone)]
pub struct ChangedFetch<'w> {
    table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<Tick>>>,
    sparse_set: Option<&'w ComponentSparseSet>,
    last_run: Tick,
    this_run: Tick,
}

/// SAFETY:
/// `fetch` accesses a single component in a readonly way.
/// This is sound because `update_component_access` add read access for that component and panics when appropriate.
/// `update_component_access` adds a `With` filter for a component.
/// This is sound because `matches_component_set` returns whether the set contains that component.
unsafe impl<T: Component> WorldQuery for Changed<T> {
    type Item<'w> = bool;
    type Fetch<'w> = ChangedFetch<'w>;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        &id: &ComponentId,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w> {
        Self::Fetch::<'w> {
            table_ticks: None,
            sparse_set: (T::STORAGE_TYPE == StorageType::SparseSet)
                .then(|| world.storages().sparse_sets.get(id).debug_checked_unwrap()),
            last_run,
            this_run,
        }
    }

    const IS_DENSE: bool = {
        match T::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        component_id: &ComponentId,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if Self::IS_DENSE {
            // SAFETY: `set_archetype`'s safety rules are a super set of the `set_table`'s ones.
            unsafe {
                Self::set_table(fetch, component_id, table);
            }
        }
    }

    #[inline]
    unsafe fn set_table<'w>(
        fetch: &mut Self::Fetch<'w>,
        &component_id: &ComponentId,
        table: &'w Table,
    ) {
        fetch.table_ticks = Some(
            Column::get_changed_ticks_slice(table.get_column(component_id).debug_checked_unwrap())
                .into(),
        );
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        match T::STORAGE_TYPE {
            StorageType::Table => {
                // SAFETY: STORAGE_TYPE = Table
                let table = unsafe { fetch.table_ticks.debug_checked_unwrap() };
                // SAFETY: The caller ensures `table_row` is in range.
                let tick = unsafe { table.get(table_row.as_usize()) };

                tick.deref().is_newer_than(fetch.last_run, fetch.this_run)
            }
            StorageType::SparseSet => {
                // SAFETY: STORAGE_TYPE = SparseSet
                let sparse_set = unsafe { &fetch.sparse_set.debug_checked_unwrap() };
                // SAFETY: The caller ensures `entity` is in range.
                let tick = unsafe {
                    ComponentSparseSet::get_changed_tick(sparse_set, entity).debug_checked_unwrap()
                };

                tick.deref().is_newer_than(fetch.last_run, fetch.this_run)
            }
        }
    }

    #[inline]
    fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
        if access.access().has_write(id) {
            panic!("$state_name<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",std::any::type_name::<T>());
        }
        access.add_read(id);
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn get_state(components: &Components) -> Option<ComponentId> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        &id: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(id)
    }
}

impl<T: Component> QueryFilter for Changed<T> {
    const IS_ARCHETYPAL: bool = false;

    #[inline(always)]
    unsafe fn filter_fetch(
        fetch: &mut Self::Fetch<'_>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        // SAFETY: The invariants are uphold by the caller.
        unsafe { Self::fetch(fetch, entity, table_row) }
    }
}

/// A marker trait to indicate that the filter works at an archetype level.
///
/// This is needed to implement [`ExactSizeIterator`] for
/// [`QueryIter`](crate::query::QueryIter) that contains archetype-level filters.
///
/// The trait must only be implemented for filters where its corresponding [`QueryFilter::IS_ARCHETYPAL`]
/// is [`prim@true`]. As such, only the [`With`] and [`Without`] filters can implement the trait.
/// [Tuples](prim@tuple) and [`Or`] filters are automatically implemented with the trait only if its containing types
/// also implement the same trait.
///
/// [`Added`] and [`Changed`] works with entities, and therefore are not archetypal. As such
/// they do not implement [`ArchetypeFilter`].
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a valid `Query` filter based on archetype information",
    label = "invalid `Query` filter",
    note = "an `ArchetypeFilter` typically uses a combination of `With<T>` and `Without<T>` statements"
)]
pub trait ArchetypeFilter: QueryFilter {}

impl<T: Component> ArchetypeFilter for With<T> {}
impl<T: Component> ArchetypeFilter for Without<T> {}

macro_rules! impl_archetype_filter_tuple {
    ($($filter: ident),*) => {
        impl<$($filter: ArchetypeFilter),*> ArchetypeFilter for ($($filter,)*) {}

        impl<$($filter: ArchetypeFilter),*> ArchetypeFilter for Or<($($filter,)*)> {}
    };
}

all_tuples!(impl_archetype_filter_tuple, 0, 15, F);
