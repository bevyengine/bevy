use crate::{
    archetype::{Archetype, Archetypes},
    bundle::Bundle,
    change_detection::{MaybeLocation, Ticks, TicksMut},
    component::{Component, ComponentId, Components, Mutable, StorageType, Tick},
    entity::{Entities, Entity, EntityLocation},
    query::{Access, DebugCheckedUnwrap, FilteredAccess, WorldQuery},
    storage::{ComponentSparseSet, Table, TableRow},
    world::{
        unsafe_world_cell::UnsafeWorldCell, EntityMut, EntityMutExcept, EntityRef, EntityRefExcept,
        FilteredEntityMut, FilteredEntityRef, Mut, Ref, World,
    },
};
use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use core::{cell::UnsafeCell, marker::PhantomData, panic::Location};
use smallvec::SmallVec;
use variadics_please::all_tuples;

/// Types that can be fetched from a [`World`] using a [`Query`].
///
/// There are many types that natively implement this trait:
///
/// - **Component references. (&T and &mut T)**
///   Fetches a component by reference (immutably or mutably).
/// - **`QueryData` tuples.**
///   If every element of a tuple implements `QueryData`, then the tuple itself also implements the same trait.
///   This enables a single `Query` to access multiple components.
///   Due to the current lack of variadic generics in Rust, the trait has been implemented for tuples from 0 to 15 elements,
///   but nesting of tuples allows infinite `WorldQuery`s.
/// - **[`Entity`].**
///   Gets the identifier of the queried entity.
/// - **[`EntityLocation`].**
///   Gets the location metadata of the queried entity.
/// - **[`EntityRef`].**
///   Read-only access to arbitrary components on the queried entity.
/// - **[`EntityMut`].**
///   Mutable access to arbitrary components on the queried entity.
/// - **[`&Archetype`](Archetype).**
///   Read-only access to the archetype-level metadata of the queried entity.
/// - **[`Option`].**
///   By default, a world query only tests entities that have the matching component types.
///   Wrapping it into an `Option` will increase the query search space, and it will return `None` if an entity doesn't satisfy the `WorldQuery`.
/// - **[`AnyOf`].**
///   Equivalent to wrapping each world query inside it into an `Option`.
/// - **[`Ref`].**
///   Similar to change detection filters but it is used as a query fetch parameter.
///   It exposes methods to check for changes to the wrapped component.
/// - **[`Has`].**
///   Returns a bool indicating whether the entity has the specified component.
///
/// Implementing the trait manually can allow for a fundamentally new type of behavior.
///
/// # Trait derivation
///
/// Query design can be easily structured by deriving `QueryData` for custom types.
/// Despite the added complexity, this approach has several advantages over using `QueryData` tuples.
/// The most relevant improvements are:
///
/// - Reusability across multiple systems.
/// - There is no need to destructure a tuple since all fields are named.
/// - Subqueries can be composed together to create a more complex query.
/// - Methods can be implemented for the query items.
/// - There is no hardcoded limit on the number of elements.
///
/// This trait can only be derived for structs, if each field also implements `QueryData`.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::QueryData;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
///
/// #[derive(QueryData)]
/// struct MyQuery {
///     entity: Entity,
///     // It is required that all reference lifetimes are explicitly annotated, just like in any
///     // struct. Each lifetime should be 'static.
///     component_a: &'static ComponentA,
///     component_b: &'static ComponentB,
/// }
///
/// fn my_system(query: Query<MyQuery>) {
///     for q in &query {
///         q.component_a;
///     }
/// }
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// ## Macro expansion
///
/// Expanding the macro will declare one or three additional structs, depending on whether or not the struct is marked as mutable.
/// For a struct named `X`, the additional structs will be:
///
/// |Struct name|`mutable` only|Description|
/// |:---:|:---:|---|
/// |`XItem`|---|The type of the query item for `X`|
/// |`XReadOnlyItem`|✓|The type of the query item for `XReadOnly`|
/// |`XReadOnly`|✓|[`ReadOnly`] variant of `X`|
///
/// ## Adding mutable references
///
/// Simply adding mutable references to a derived `QueryData` will result in a compilation error:
///
/// ```compile_fail
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::QueryData;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// #[derive(QueryData)]
/// struct CustomQuery {
///     component_a: &'static mut ComponentA,
/// }
/// ```
///
/// To grant mutable access to components, the struct must be marked with the `#[query_data(mutable)]` attribute.
/// This will also create three more structs that will be used for accessing the query immutably (see table above).
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::QueryData;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// #[derive(QueryData)]
/// #[query_data(mutable)]
/// struct CustomQuery {
///     component_a: &'static mut ComponentA,
/// }
/// ```
///
/// ## Adding methods to query items
///
/// It is possible to add methods to query items in order to write reusable logic about related components.
/// This will often make systems more readable because low level logic is moved out from them.
/// It is done by adding `impl` blocks with methods for the `-Item` or `-ReadOnlyItem` generated structs.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::QueryData;
/// #
/// #[derive(Component)]
/// struct Health(f32);
///
/// #[derive(Component)]
/// struct Buff(f32);
///
/// #[derive(QueryData)]
/// #[query_data(mutable)]
/// struct HealthQuery {
///     health: &'static mut Health,
///     buff: Option<&'static mut Buff>,
/// }
///
/// // `HealthQueryItem` is only available when accessing the query with mutable methods.
/// impl<'w> HealthQueryItem<'w> {
///     fn damage(&mut self, value: f32) {
///         self.health.0 -= value;
///     }
///
///     fn total(&self) -> f32 {
///         self.health.0 + self.buff.as_deref().map_or(0.0, |Buff(buff)| *buff)
///     }
/// }
///
/// // `HealthQueryReadOnlyItem` is only available when accessing the query with immutable methods.
/// impl<'w> HealthQueryReadOnlyItem<'w> {
///     fn total(&self) -> f32 {
///         self.health.0 + self.buff.map_or(0.0, |Buff(buff)| *buff)
///     }
/// }
///
/// fn my_system(mut health_query: Query<HealthQuery>) {
///     // The item returned by the iterator is of type `HealthQueryReadOnlyItem`.
///     for health in health_query.iter() {
///         println!("Total: {}", health.total());
///     }
///     // The item returned by the iterator is of type `HealthQueryItem`.
///     for mut health in &mut health_query {
///         health.damage(1.0);
///         println!("Total (mut): {}", health.total());
///     }
/// }
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// ## Deriving traits for query items
///
/// The `QueryData` derive macro does not automatically implement the traits of the struct to the query item types.
/// Something similar can be done by using the `#[query_data(derive(...))]` attribute.
/// This will apply the listed derivable traits to the query item structs.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::QueryData;
/// #
/// # #[derive(Component, Debug)]
/// # struct ComponentA;
/// #
/// #[derive(QueryData)]
/// #[query_data(mutable, derive(Debug))]
/// struct CustomQuery {
///     component_a: &'static ComponentA,
/// }
///
/// // This function statically checks that `T` implements `Debug`.
/// fn assert_debug<T: std::fmt::Debug>() {}
///
/// assert_debug::<CustomQueryItem>();
/// assert_debug::<CustomQueryReadOnlyItem>();
/// ```
///
/// ## Query composition
///
/// It is possible to use any `QueryData` as a field of another one.
/// This means that a `QueryData` can also be used as a subquery, potentially in multiple places.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::QueryData;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # #[derive(Component)]
/// # struct ComponentC;
/// #
/// #[derive(QueryData)]
/// struct SubQuery {
///     component_a: &'static ComponentA,
///     component_b: &'static ComponentB,
/// }
///
/// #[derive(QueryData)]
/// struct MyQuery {
///     subquery: SubQuery,
///     component_c: &'static ComponentC,
/// }
/// ```
///
/// # Generic Queries
///
/// When writing generic code, it is often necessary to use [`PhantomData`]
/// to constrain type parameters. Since `QueryData` is implemented for all
/// `PhantomData<T>` types, this pattern can be used with this macro.
///
/// ```
/// # use bevy_ecs::{prelude::*, query::QueryData};
/// # use std::marker::PhantomData;
/// #[derive(QueryData)]
/// pub struct GenericQuery<T> {
///     id: Entity,
///     marker: PhantomData<T>,
/// }
/// # fn my_system(q: Query<GenericQuery<()>>) {}
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// # Safety
///
/// - Component access of `Self::ReadOnly` must be a subset of `Self`
///   and `Self::ReadOnly` must match exactly the same archetypes/tables as `Self`
/// - `IS_READ_ONLY` must be `true` if and only if `Self: ReadOnlyQueryData`
///
/// [`Query`]: crate::system::Query
/// [`ReadOnly`]: Self::ReadOnly
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not valid to request as data in a `Query`",
    label = "invalid `Query` data",
    note = "if `{Self}` is a component type, try using `&{Self}` or `&mut {Self}`"
)]
pub unsafe trait QueryData: WorldQuery {
    /// True if this query is read-only and may not perform mutable access.
    const IS_READ_ONLY: bool;

    /// The read-only variant of this [`QueryData`], which satisfies the [`ReadOnlyQueryData`] trait.
    type ReadOnly: ReadOnlyQueryData<State = <Self as WorldQuery>::State>;

    /// The item returned by this [`WorldQuery`]
    /// This will be the data retrieved by the query,
    /// and is visible to the end user when calling e.g. `Query<Self>::get`.
    type Item<'a>;

    /// This function manually implements subtyping for the query items.
    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort>;

    /// Fetch [`Self::Item`](`QueryData::Item`) for either the given `entity` in the current [`Table`],
    /// or for the given `entity` in the current [`Archetype`]. This must always be called after
    /// [`WorldQuery::set_table`] with a `table_row` in the range of the current [`Table`] or after
    /// [`WorldQuery::set_archetype`]  with an `entity` in the current archetype.
    /// Accesses components registered in [`WorldQuery::update_component_access`].
    ///
    /// # Safety
    ///
    /// - Must always be called _after_ [`WorldQuery::set_table`] or [`WorldQuery::set_archetype`]. `entity` and
    ///   `table_row` must be in the range of the current table and archetype.
    /// - There must not be simultaneous conflicting component access registered in `update_component_access`.
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w>;
}

/// A [`QueryData`] that is read only.
///
/// # Safety
///
/// This must only be implemented for read-only [`QueryData`]'s.
pub unsafe trait ReadOnlyQueryData: QueryData<ReadOnly = Self> {}

/// The item type returned when a [`WorldQuery`] is iterated over
pub type QueryItem<'w, Q> = <Q as QueryData>::Item<'w>;
/// The read-only variant of the item type returned when a [`QueryData`] is iterated over immutably
pub type ROQueryItem<'w, D> = QueryItem<'w, <D as QueryData>::ReadOnly>;

/// SAFETY:
/// `update_component_access` and `update_archetype_component_access` do nothing.
/// This is sound because `fetch` does not access components.
unsafe impl WorldQuery for Entity {
    type Fetch<'w> = ();
    type State = ();

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(_: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {}

    unsafe fn init_fetch<'w>(
        _world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
    }

    const IS_DENSE: bool = true;

    #[inline]
    unsafe fn set_archetype<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
    }

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn init_state(_world: &mut World) {}

    fn get_state(_components: &Components) -> Option<()> {
        Some(())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl QueryData for Entity {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;

    type Item<'w> = Entity;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        _fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        entity
    }
}

/// SAFETY: access is read only
unsafe impl ReadOnlyQueryData for Entity {}

/// SAFETY:
/// `update_component_access` and `update_archetype_component_access` do nothing.
/// This is sound because `fetch` does not access components.
unsafe impl WorldQuery for EntityLocation {
    type Fetch<'w> = &'w Entities;
    type State = ();

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        world.entities()
    }

    // This is set to true to avoid forcing archetypal iteration in compound queries, is likely to be slower
    // in most practical use case.
    const IS_DENSE: bool = true;

    #[inline]
    unsafe fn set_archetype<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
    }

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn init_state(_world: &mut World) {}

    fn get_state(_components: &Components) -> Option<()> {
        Some(())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl QueryData for EntityLocation {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = EntityLocation;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: `fetch` must be called with an entity that exists in the world
        unsafe { fetch.get(entity).debug_checked_unwrap() }
    }
}

/// SAFETY: access is read only
unsafe impl ReadOnlyQueryData for EntityLocation {}

/// SAFETY:
/// `fetch` accesses all components in a readonly way.
/// This is sound because `update_component_access` and `update_archetype_component_access` set read access for all components and panic when appropriate.
/// Filters are unchanged.
unsafe impl<'a> WorldQuery for EntityRef<'a> {
    type Fetch<'w> = UnsafeWorldCell<'w>;
    type State = ();

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        world
    }

    const IS_DENSE: bool = true;

    #[inline]
    unsafe fn set_archetype<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
    }

    fn update_component_access(_state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_any_component_write(),
            "EntityRef conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
        );
        access.read_all_components();
    }

    fn init_state(_world: &mut World) {}

    fn get_state(_components: &Components) -> Option<()> {
        Some(())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl<'a> QueryData for EntityRef<'a> {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = EntityRef<'w>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        world: &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: `fetch` must be called with an entity that exists in the world
        let cell = unsafe { world.get_entity(entity).debug_checked_unwrap() };
        // SAFETY: Read-only access to every component has been registered.
        unsafe { EntityRef::new(cell) }
    }
}

/// SAFETY: access is read only
unsafe impl ReadOnlyQueryData for EntityRef<'_> {}

/// SAFETY: The accesses of `Self::ReadOnly` are a subset of the accesses of `Self`
unsafe impl<'a> WorldQuery for EntityMut<'a> {
    type Fetch<'w> = UnsafeWorldCell<'w>;
    type State = ();

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        world
    }

    const IS_DENSE: bool = true;

    #[inline]
    unsafe fn set_archetype<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
    }

    fn update_component_access(_state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_any_component_read(),
            "EntityMut conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
        );
        access.write_all_components();
    }

    fn init_state(_world: &mut World) {}

    fn get_state(_components: &Components) -> Option<()> {
        Some(())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: access of `EntityRef` is a subset of `EntityMut`
unsafe impl<'a> QueryData for EntityMut<'a> {
    const IS_READ_ONLY: bool = false;
    type ReadOnly = EntityRef<'a>;
    type Item<'w> = EntityMut<'w>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        world: &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: `fetch` must be called with an entity that exists in the world
        let cell = unsafe { world.get_entity(entity).debug_checked_unwrap() };
        // SAFETY: mutable access to every component has been registered.
        unsafe { EntityMut::new(cell) }
    }
}

/// SAFETY: The accesses of `Self::ReadOnly` are a subset of the accesses of `Self`
unsafe impl<'a> WorldQuery for FilteredEntityRef<'a> {
    type Fetch<'w> = (UnsafeWorldCell<'w>, Access<ComponentId>);
    type State = FilteredAccess<ComponentId>;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    const IS_DENSE: bool = false;

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        let mut access = Access::default();
        access.read_all_components();
        (world, access)
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        _: &'w Archetype,
        _table: &Table,
    ) {
        fetch.1.clone_from(&state.access);
    }

    #[inline]
    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, _: &'w Table) {
        fetch.1.clone_from(&state.access);
    }

    #[inline]
    fn set_access<'w>(state: &mut Self::State, access: &FilteredAccess<ComponentId>) {
        state.clone_from(access);
        state.access_mut().clear_writes();
    }

    fn update_component_access(
        state: &Self::State,
        filtered_access: &mut FilteredAccess<ComponentId>,
    ) {
        assert!(
            filtered_access.access().is_compatible(&state.access),
            "FilteredEntityRef conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
        );
        filtered_access.access.extend(&state.access);
    }

    fn init_state(_world: &mut World) -> Self::State {
        FilteredAccess::default()
    }

    fn get_state(_components: &Components) -> Option<Self::State> {
        Some(FilteredAccess::default())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl<'a> QueryData for FilteredEntityRef<'a> {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = FilteredEntityRef<'w>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        (world, access): &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: `fetch` must be called with an entity that exists in the world
        let cell = unsafe { world.get_entity(entity).debug_checked_unwrap() };
        // SAFETY: mutable access to every component has been registered.
        unsafe { FilteredEntityRef::new(cell, access.clone()) }
    }
}

/// SAFETY: Access is read-only.
unsafe impl ReadOnlyQueryData for FilteredEntityRef<'_> {}

/// SAFETY: The accesses of `Self::ReadOnly` are a subset of the accesses of `Self`
unsafe impl<'a> WorldQuery for FilteredEntityMut<'a> {
    type Fetch<'w> = (UnsafeWorldCell<'w>, Access<ComponentId>);
    type State = FilteredAccess<ComponentId>;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    const IS_DENSE: bool = false;

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        let mut access = Access::default();
        access.write_all_components();
        (world, access)
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        _: &'w Archetype,
        _table: &Table,
    ) {
        fetch.1.clone_from(&state.access);
    }

    #[inline]
    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, _: &'w Table) {
        fetch.1.clone_from(&state.access);
    }

    #[inline]
    fn set_access<'w>(state: &mut Self::State, access: &FilteredAccess<ComponentId>) {
        state.clone_from(access);
    }

    fn update_component_access(
        state: &Self::State,
        filtered_access: &mut FilteredAccess<ComponentId>,
    ) {
        assert!(
            filtered_access.access().is_compatible(&state.access),
            "FilteredEntityMut conflicts with a previous access in this query. Exclusive access cannot coincide with any other accesses.",
        );
        filtered_access.access.extend(&state.access);
    }

    fn init_state(_world: &mut World) -> Self::State {
        FilteredAccess::default()
    }

    fn get_state(_components: &Components) -> Option<Self::State> {
        Some(FilteredAccess::default())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: access of `FilteredEntityRef` is a subset of `FilteredEntityMut`
unsafe impl<'a> QueryData for FilteredEntityMut<'a> {
    const IS_READ_ONLY: bool = false;
    type ReadOnly = FilteredEntityRef<'a>;
    type Item<'w> = FilteredEntityMut<'w>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        (world, access): &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        // SAFETY: `fetch` must be called with an entity that exists in the world
        let cell = unsafe { world.get_entity(entity).debug_checked_unwrap() };
        // SAFETY: mutable access to every component has been registered.
        unsafe { FilteredEntityMut::new(cell, access.clone()) }
    }
}

/// SAFETY: `EntityRefExcept` guards access to all components in the bundle `B`
/// and populates `Access` values so that queries that conflict with this access
/// are rejected.
unsafe impl<'a, B> WorldQuery for EntityRefExcept<'a, B>
where
    B: Bundle,
{
    type Fetch<'w> = UnsafeWorldCell<'w>;
    type State = SmallVec<[ComponentId; 4]>;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _: &Self::State,
        _: Tick,
        _: Tick,
    ) -> Self::Fetch<'w> {
        world
    }

    const IS_DENSE: bool = true;

    unsafe fn set_archetype<'w>(
        _: &mut Self::Fetch<'w>,
        _: &Self::State,
        _: &'w Archetype,
        _: &'w Table,
    ) {
    }

    unsafe fn set_table<'w>(_: &mut Self::Fetch<'w>, _: &Self::State, _: &'w Table) {}

    fn update_component_access(
        state: &Self::State,
        filtered_access: &mut FilteredAccess<ComponentId>,
    ) {
        let mut my_access = Access::new();
        my_access.read_all_components();
        for id in state {
            my_access.remove_component_read(*id);
        }

        let access = filtered_access.access_mut();
        assert!(
            access.is_compatible(&my_access),
            "`EntityRefExcept<{}>` conflicts with a previous access in this query.",
            core::any::type_name::<B>(),
        );
        access.extend(&my_access);
    }

    fn init_state(world: &mut World) -> Self::State {
        Self::get_state(world.components()).unwrap()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        let mut ids = SmallVec::new();
        B::get_component_ids(components, &mut |maybe_id| {
            if let Some(id) = maybe_id {
                ids.push(id);
            }
        });
        Some(ids)
    }

    fn matches_component_set(_: &Self::State, _: &impl Fn(ComponentId) -> bool) -> bool {
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`.
unsafe impl<'a, B> QueryData for EntityRefExcept<'a, B>
where
    B: Bundle,
{
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = EntityRefExcept<'w, B>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    unsafe fn fetch<'w>(
        world: &mut Self::Fetch<'w>,
        entity: Entity,
        _: TableRow,
    ) -> Self::Item<'w> {
        let cell = world.get_entity(entity).unwrap();
        EntityRefExcept::new(cell)
    }
}

/// SAFETY: `EntityRefExcept` enforces read-only access to its contained
/// components.
unsafe impl<'a, B> ReadOnlyQueryData for EntityRefExcept<'a, B> where B: Bundle {}

/// SAFETY: `EntityMutExcept` guards access to all components in the bundle `B`
/// and populates `Access` values so that queries that conflict with this access
/// are rejected.
unsafe impl<'a, B> WorldQuery for EntityMutExcept<'a, B>
where
    B: Bundle,
{
    type Fetch<'w> = UnsafeWorldCell<'w>;
    type State = SmallVec<[ComponentId; 4]>;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _: &Self::State,
        _: Tick,
        _: Tick,
    ) -> Self::Fetch<'w> {
        world
    }

    const IS_DENSE: bool = true;

    unsafe fn set_archetype<'w>(
        _: &mut Self::Fetch<'w>,
        _: &Self::State,
        _: &'w Archetype,
        _: &'w Table,
    ) {
    }

    unsafe fn set_table<'w>(_: &mut Self::Fetch<'w>, _: &Self::State, _: &'w Table) {}

    fn update_component_access(
        state: &Self::State,
        filtered_access: &mut FilteredAccess<ComponentId>,
    ) {
        let mut my_access = Access::new();
        my_access.write_all_components();
        for id in state {
            my_access.remove_component_read(*id);
        }

        let access = filtered_access.access_mut();
        assert!(
            access.is_compatible(&my_access),
            "`EntityMutExcept<{}>` conflicts with a previous access in this query.",
            core::any::type_name::<B>()
        );
        access.extend(&my_access);
    }

    fn init_state(world: &mut World) -> Self::State {
        Self::get_state(world.components()).unwrap()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        let mut ids = SmallVec::new();
        B::get_component_ids(components, &mut |maybe_id| {
            if let Some(id) = maybe_id {
                ids.push(id);
            }
        });
        Some(ids)
    }

    fn matches_component_set(_: &Self::State, _: &impl Fn(ComponentId) -> bool) -> bool {
        true
    }
}

/// SAFETY: All accesses that `EntityRefExcept` provides are also accesses that
/// `EntityMutExcept` provides.
unsafe impl<'a, B> QueryData for EntityMutExcept<'a, B>
where
    B: Bundle,
{
    const IS_READ_ONLY: bool = false;
    type ReadOnly = EntityRefExcept<'a, B>;
    type Item<'w> = EntityMutExcept<'w, B>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    unsafe fn fetch<'w>(
        world: &mut Self::Fetch<'w>,
        entity: Entity,
        _: TableRow,
    ) -> Self::Item<'w> {
        let cell = world.get_entity(entity).unwrap();
        EntityMutExcept::new(cell)
    }
}

/// SAFETY:
/// `update_component_access` and `update_archetype_component_access` do nothing.
/// This is sound because `fetch` does not access components.
unsafe impl WorldQuery for &Archetype {
    type Fetch<'w> = (&'w Entities, &'w Archetypes);
    type State = ();

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        (world.entities(), world.archetypes())
    }

    // This could probably be a non-dense query and just set a Option<&Archetype> fetch value in
    // set_archetypes, but forcing archetypal iteration is likely to be slower in any compound query.
    const IS_DENSE: bool = true;

    #[inline]
    unsafe fn set_archetype<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
    }

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn init_state(_world: &mut World) {}

    fn get_state(_components: &Components) -> Option<()> {
        Some(())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl QueryData for &Archetype {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = &'w Archetype;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        let (entities, archetypes) = *fetch;
        // SAFETY: `fetch` must be called with an entity that exists in the world
        let location = unsafe { entities.get(entity).debug_checked_unwrap() };
        // SAFETY: The assigned archetype for a living entity must always be valid.
        unsafe { archetypes.get(location.archetype_id).debug_checked_unwrap() }
    }
}

/// SAFETY: access is read only
unsafe impl ReadOnlyQueryData for &Archetype {}

/// The [`WorldQuery::Fetch`] type for `& T`.
pub struct ReadFetch<'w, T: Component> {
    components: StorageSwitch<
        T,
        // T::STORAGE_TYPE = StorageType::Table
        Option<ThinSlicePtr<'w, UnsafeCell<T>>>,
        // T::STORAGE_TYPE = StorageType::SparseSet
        Option<&'w ComponentSparseSet>,
    >,
}

impl<T: Component> Clone for ReadFetch<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Component> Copy for ReadFetch<'_, T> {}

/// SAFETY:
/// `fetch` accesses a single component in a readonly way.
/// This is sound because `update_component_access` and `update_archetype_component_access` add read access for that component and panic when appropriate.
/// `update_component_access` adds a `With` filter for a component.
/// This is sound because `matches_component_set` returns whether the set contains that component.
unsafe impl<T: Component> WorldQuery for &T {
    type Fetch<'w> = ReadFetch<'w, T>;
    type State = ComponentId;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        &component_id: &ComponentId,
        _last_run: Tick,
        _this_run: Tick,
    ) -> ReadFetch<'w, T> {
        ReadFetch {
            components: StorageSwitch::new(
                || None,
                || {
                    // SAFETY: The underlying type associated with `component_id` is `T`,
                    // which we are allowed to access since we registered it in `update_archetype_component_access`.
                    // Note that we do not actually access any components in this function, we just get a shared
                    // reference to the sparse set, which is used to access the components in `Self::fetch`.
                    unsafe { world.storages().sparse_sets.get(component_id) }
                },
            ),
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
        fetch: &mut ReadFetch<'w, T>,
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
        fetch: &mut ReadFetch<'w, T>,
        &component_id: &ComponentId,
        table: &'w Table,
    ) {
        let table_data = Some(
            table
                .get_data_slice_for(component_id)
                .debug_checked_unwrap()
                .into(),
        );
        // SAFETY: set_table is only called when T::STORAGE_TYPE = StorageType::Table
        unsafe { fetch.components.set_table(table_data) };
    }

    fn update_component_access(
        &component_id: &ComponentId,
        access: &mut FilteredAccess<ComponentId>,
    ) {
        assert!(
            !access.access().has_component_write(component_id),
            "&{} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
            core::any::type_name::<T>(),
        );
        access.add_component_read(component_id);
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.register_component::<T>()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        &state: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state)
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl<T: Component> QueryData for &T {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = &'w T;

    fn shrink<'wlong: 'wshort, 'wshort>(item: &'wlong T) -> &'wshort T {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        fetch.components.extract(
            |table| {
                // SAFETY: set_table was previously called
                let table = unsafe { table.debug_checked_unwrap() };
                // SAFETY: Caller ensures `table_row` is in range.
                let item = unsafe { table.get(table_row.as_usize()) };
                item.deref()
            },
            |sparse_set| {
                // SAFETY: Caller ensures `entity` is in range.
                let item = unsafe {
                    sparse_set
                        .debug_checked_unwrap()
                        .get(entity)
                        .debug_checked_unwrap()
                };
                item.deref()
            },
        )
    }
}

/// SAFETY: access is read only
unsafe impl<T: Component> ReadOnlyQueryData for &T {}

#[doc(hidden)]
pub struct RefFetch<'w, T: Component> {
    components: StorageSwitch<
        T,
        // T::STORAGE_TYPE = StorageType::Table
        Option<(
            ThinSlicePtr<'w, UnsafeCell<T>>,
            ThinSlicePtr<'w, UnsafeCell<Tick>>,
            ThinSlicePtr<'w, UnsafeCell<Tick>>,
            MaybeLocation<ThinSlicePtr<'w, UnsafeCell<&'static Location<'static>>>>,
        )>,
        // T::STORAGE_TYPE = StorageType::SparseSet
        // Can be `None` when the component has never been inserted
        Option<&'w ComponentSparseSet>,
    >,
    last_run: Tick,
    this_run: Tick,
}

impl<T: Component> Clone for RefFetch<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Component> Copy for RefFetch<'_, T> {}

/// SAFETY:
/// `fetch` accesses a single component in a readonly way.
/// This is sound because `update_component_access` and `update_archetype_component_access` add read access for that component and panic when appropriate.
/// `update_component_access` adds a `With` filter for a component.
/// This is sound because `matches_component_set` returns whether the set contains that component.
unsafe impl<'__w, T: Component> WorldQuery for Ref<'__w, T> {
    type Fetch<'w> = RefFetch<'w, T>;
    type State = ComponentId;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        &component_id: &ComponentId,
        last_run: Tick,
        this_run: Tick,
    ) -> RefFetch<'w, T> {
        RefFetch {
            components: StorageSwitch::new(
                || None,
                || {
                    // SAFETY: The underlying type associated with `component_id` is `T`,
                    // which we are allowed to access since we registered it in `update_archetype_component_access`.
                    // Note that we do not actually access any components in this function, we just get a shared
                    // reference to the sparse set, which is used to access the components in `Self::fetch`.
                    unsafe { world.storages().sparse_sets.get(component_id) }
                },
            ),
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
        fetch: &mut RefFetch<'w, T>,
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
        fetch: &mut RefFetch<'w, T>,
        &component_id: &ComponentId,
        table: &'w Table,
    ) {
        let column = table.get_column(component_id).debug_checked_unwrap();
        let table_data = Some((
            column.get_data_slice(table.entity_count()).into(),
            column.get_added_ticks_slice(table.entity_count()).into(),
            column.get_changed_ticks_slice(table.entity_count()).into(),
            column
                .get_changed_by_slice(table.entity_count())
                .map(Into::into),
        ));
        // SAFETY: set_table is only called when T::STORAGE_TYPE = StorageType::Table
        unsafe { fetch.components.set_table(table_data) };
    }

    fn update_component_access(
        &component_id: &ComponentId,
        access: &mut FilteredAccess<ComponentId>,
    ) {
        assert!(
            !access.access().has_component_write(component_id),
            "&{} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
            core::any::type_name::<T>(),
        );
        access.add_component_read(component_id);
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.register_component::<T>()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        &state: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state)
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl<'__w, T: Component> QueryData for Ref<'__w, T> {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = Ref<'w, T>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Ref<'wlong, T>) -> Ref<'wshort, T> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        fetch.components.extract(
            |table| {
                // SAFETY: set_table was previously called
                let (table_components, added_ticks, changed_ticks, callers) =
                    unsafe { table.debug_checked_unwrap() };

                // SAFETY: The caller ensures `table_row` is in range.
                let component = unsafe { table_components.get(table_row.as_usize()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let added = unsafe { added_ticks.get(table_row.as_usize()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let changed = unsafe { changed_ticks.get(table_row.as_usize()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let caller = callers.map(|callers| unsafe { callers.get(table_row.as_usize()) });

                Ref {
                    value: component.deref(),
                    ticks: Ticks {
                        added: added.deref(),
                        changed: changed.deref(),
                        this_run: fetch.this_run,
                        last_run: fetch.last_run,
                    },
                    changed_by: caller.map(|caller| caller.deref()),
                }
            },
            |sparse_set| {
                // SAFETY: The caller ensures `entity` is in range and has the component.
                let (component, ticks, caller) = unsafe {
                    sparse_set
                        .debug_checked_unwrap()
                        .get_with_ticks(entity)
                        .debug_checked_unwrap()
                };

                Ref {
                    value: component.deref(),
                    ticks: Ticks::from_tick_cells(ticks, fetch.last_run, fetch.this_run),
                    changed_by: caller.map(|caller| caller.deref()),
                }
            },
        )
    }
}

/// SAFETY: access is read only
unsafe impl<'__w, T: Component> ReadOnlyQueryData for Ref<'__w, T> {}

/// The [`WorldQuery::Fetch`] type for `&mut T`.
pub struct WriteFetch<'w, T: Component> {
    components: StorageSwitch<
        T,
        // T::STORAGE_TYPE = StorageType::Table
        Option<(
            ThinSlicePtr<'w, UnsafeCell<T>>,
            ThinSlicePtr<'w, UnsafeCell<Tick>>,
            ThinSlicePtr<'w, UnsafeCell<Tick>>,
            MaybeLocation<ThinSlicePtr<'w, UnsafeCell<&'static Location<'static>>>>,
        )>,
        // T::STORAGE_TYPE = StorageType::SparseSet
        // Can be `None` when the component has never been inserted
        Option<&'w ComponentSparseSet>,
    >,
    last_run: Tick,
    this_run: Tick,
}

impl<T: Component> Clone for WriteFetch<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Component> Copy for WriteFetch<'_, T> {}

/// SAFETY:
/// `fetch` accesses a single component mutably.
/// This is sound because `update_component_access` and `update_archetype_component_access` add write access for that component and panic when appropriate.
/// `update_component_access` adds a `With` filter for a component.
/// This is sound because `matches_component_set` returns whether the set contains that component.
unsafe impl<'__w, T: Component> WorldQuery for &'__w mut T {
    type Fetch<'w> = WriteFetch<'w, T>;
    type State = ComponentId;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        &component_id: &ComponentId,
        last_run: Tick,
        this_run: Tick,
    ) -> WriteFetch<'w, T> {
        WriteFetch {
            components: StorageSwitch::new(
                || None,
                || {
                    // SAFETY: The underlying type associated with `component_id` is `T`,
                    // which we are allowed to access since we registered it in `update_archetype_component_access`.
                    // Note that we do not actually access any components in this function, we just get a shared
                    // reference to the sparse set, which is used to access the components in `Self::fetch`.
                    unsafe { world.storages().sparse_sets.get(component_id) }
                },
            ),
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
        fetch: &mut WriteFetch<'w, T>,
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
        fetch: &mut WriteFetch<'w, T>,
        &component_id: &ComponentId,
        table: &'w Table,
    ) {
        let column = table.get_column(component_id).debug_checked_unwrap();
        let table_data = Some((
            column.get_data_slice(table.entity_count()).into(),
            column.get_added_ticks_slice(table.entity_count()).into(),
            column.get_changed_ticks_slice(table.entity_count()).into(),
            column
                .get_changed_by_slice(table.entity_count())
                .map(Into::into),
        ));
        // SAFETY: set_table is only called when T::STORAGE_TYPE = StorageType::Table
        unsafe { fetch.components.set_table(table_data) };
    }

    fn update_component_access(
        &component_id: &ComponentId,
        access: &mut FilteredAccess<ComponentId>,
    ) {
        assert!(
            !access.access().has_component_read(component_id),
            "&mut {} conflicts with a previous access in this query. Mutable component access must be unique.",
            core::any::type_name::<T>(),
        );
        access.add_component_write(component_id);
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.register_component::<T>()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        &state: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state)
    }
}

/// SAFETY: access of `&T` is a subset of `&mut T`
unsafe impl<'__w, T: Component<Mutability = Mutable>> QueryData for &'__w mut T {
    const IS_READ_ONLY: bool = false;
    type ReadOnly = &'__w T;
    type Item<'w> = Mut<'w, T>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Mut<'wlong, T>) -> Mut<'wshort, T> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        fetch.components.extract(
            |table| {
                // SAFETY: set_table was previously called
                let (table_components, added_ticks, changed_ticks, callers) =
                    unsafe { table.debug_checked_unwrap() };

                // SAFETY: The caller ensures `table_row` is in range.
                let component = unsafe { table_components.get(table_row.as_usize()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let added = unsafe { added_ticks.get(table_row.as_usize()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let changed = unsafe { changed_ticks.get(table_row.as_usize()) };
                // SAFETY: The caller ensures `table_row` is in range.
                let caller = callers.map(|callers| unsafe { callers.get(table_row.as_usize()) });

                Mut {
                    value: component.deref_mut(),
                    ticks: TicksMut {
                        added: added.deref_mut(),
                        changed: changed.deref_mut(),
                        this_run: fetch.this_run,
                        last_run: fetch.last_run,
                    },
                    changed_by: caller.map(|caller| caller.deref_mut()),
                }
            },
            |sparse_set| {
                // SAFETY: The caller ensures `entity` is in range and has the component.
                let (component, ticks, caller) = unsafe {
                    sparse_set
                        .debug_checked_unwrap()
                        .get_with_ticks(entity)
                        .debug_checked_unwrap()
                };

                Mut {
                    value: component.assert_unique().deref_mut(),
                    ticks: TicksMut::from_tick_cells(ticks, fetch.last_run, fetch.this_run),
                    changed_by: caller.map(|caller| caller.deref_mut()),
                }
            },
        )
    }
}

/// When `Mut<T>` is used in a query, it will be converted to `Ref<T>` when transformed into its read-only form, providing access to change detection methods.
///
/// By contrast `&mut T` will result in a `Mut<T>` item in mutable form to record mutations, but result in a bare `&T` in read-only form.
///
/// SAFETY:
/// `fetch` accesses a single component mutably.
/// This is sound because `update_component_access` and `update_archetype_component_access` add write access for that component and panic when appropriate.
/// `update_component_access` adds a `With` filter for a component.
/// This is sound because `matches_component_set` returns whether the set contains that component.
unsafe impl<'__w, T: Component> WorldQuery for Mut<'__w, T> {
    type Fetch<'w> = WriteFetch<'w, T>;
    type State = ComponentId;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    #[inline]
    // Forwarded to `&mut T`
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        state: &ComponentId,
        last_run: Tick,
        this_run: Tick,
    ) -> WriteFetch<'w, T> {
        <&mut T as WorldQuery>::init_fetch(world, state, last_run, this_run)
    }

    // Forwarded to `&mut T`
    const IS_DENSE: bool = <&mut T as WorldQuery>::IS_DENSE;

    #[inline]
    // Forwarded to `&mut T`
    unsafe fn set_archetype<'w>(
        fetch: &mut WriteFetch<'w, T>,
        state: &ComponentId,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        <&mut T as WorldQuery>::set_archetype(fetch, state, archetype, table);
    }

    #[inline]
    // Forwarded to `&mut T`
    unsafe fn set_table<'w>(fetch: &mut WriteFetch<'w, T>, state: &ComponentId, table: &'w Table) {
        <&mut T as WorldQuery>::set_table(fetch, state, table);
    }

    // NOT forwarded to `&mut T`
    fn update_component_access(
        &component_id: &ComponentId,
        access: &mut FilteredAccess<ComponentId>,
    ) {
        // Update component access here instead of in `<&mut T as WorldQuery>` to avoid erroneously referencing
        // `&mut T` in error message.
        assert!(
            !access.access().has_component_read(component_id),
            "Mut<{}> conflicts with a previous access in this query. Mutable component access mut be unique.",
            core::any::type_name::<T>(),
        );
        access.add_component_write(component_id);
    }

    // Forwarded to `&mut T`
    fn init_state(world: &mut World) -> ComponentId {
        <&mut T as WorldQuery>::init_state(world)
    }

    // Forwarded to `&mut T`
    fn get_state(components: &Components) -> Option<ComponentId> {
        <&mut T as WorldQuery>::get_state(components)
    }

    // Forwarded to `&mut T`
    fn matches_component_set(
        state: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        <&mut T as WorldQuery>::matches_component_set(state, set_contains_id)
    }
}

// SAFETY: access of `Ref<T>` is a subset of `Mut<T>`
unsafe impl<'__w, T: Component<Mutability = Mutable>> QueryData for Mut<'__w, T> {
    const IS_READ_ONLY: bool = false;
    type ReadOnly = Ref<'__w, T>;
    type Item<'w> = Mut<'w, T>;

    // Forwarded to `&mut T`
    fn shrink<'wlong: 'wshort, 'wshort>(item: Mut<'wlong, T>) -> Mut<'wshort, T> {
        <&mut T as QueryData>::shrink(item)
    }

    #[inline(always)]
    // Forwarded to `&mut T`
    unsafe fn fetch<'w>(
        // Rust complains about lifetime bounds not matching the trait if I directly use `WriteFetch<'w, T>` right here.
        // But it complains nowhere else in the entire trait implementation.
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Mut<'w, T> {
        <&mut T as QueryData>::fetch(fetch, entity, table_row)
    }
}

#[doc(hidden)]
pub struct OptionFetch<'w, T: WorldQuery> {
    fetch: T::Fetch<'w>,
    matches: bool,
}

impl<T: WorldQuery> Clone for OptionFetch<'_, T> {
    fn clone(&self) -> Self {
        Self {
            fetch: self.fetch.clone(),
            matches: self.matches,
        }
    }
}

/// SAFETY:
/// `fetch` might access any components that `T` accesses.
/// This is sound because `update_component_access` and `update_archetype_component_access` add the same accesses as `T`.
/// Filters are unchanged.
unsafe impl<T: WorldQuery> WorldQuery for Option<T> {
    type Fetch<'w> = OptionFetch<'w, T>;
    type State = T::State;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        OptionFetch {
            fetch: T::shrink_fetch(fetch.fetch),
            matches: fetch.matches,
        }
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        state: &T::State,
        last_run: Tick,
        this_run: Tick,
    ) -> OptionFetch<'w, T> {
        OptionFetch {
            // SAFETY: The invariants are upheld by the caller.
            fetch: unsafe { T::init_fetch(world, state, last_run, this_run) },
            matches: false,
        }
    }

    const IS_DENSE: bool = T::IS_DENSE;

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut OptionFetch<'w, T>,
        state: &T::State,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        fetch.matches = T::matches_component_set(state, &|id| archetype.contains(id));
        if fetch.matches {
            // SAFETY: The invariants are upheld by the caller.
            unsafe {
                T::set_archetype(&mut fetch.fetch, state, archetype, table);
            }
        }
    }

    #[inline]
    unsafe fn set_table<'w>(fetch: &mut OptionFetch<'w, T>, state: &T::State, table: &'w Table) {
        fetch.matches = T::matches_component_set(state, &|id| table.has_column(id));
        if fetch.matches {
            // SAFETY: The invariants are upheld by the caller.
            unsafe {
                T::set_table(&mut fetch.fetch, state, table);
            }
        }
    }

    fn update_component_access(state: &T::State, access: &mut FilteredAccess<ComponentId>) {
        // FilteredAccess::add_[write,read] adds the component to the `with` filter.
        // Those methods are called on `access` in `T::update_component_access`.
        // But in `Option<T>`, we specifically don't filter on `T`,
        // since `(Option<T>, &OtherComponent)` should be a valid item, even
        // if `Option<T>` is `None`.
        //
        // We pass a clone of the `FilteredAccess` to `T`, and only update the `Access`
        // using `extend_access` so that we can apply `T`'s component_access
        // without updating the `with` filters of `access`.
        let mut intermediate = access.clone();
        T::update_component_access(state, &mut intermediate);
        access.extend_access(&intermediate);
    }

    fn init_state(world: &mut World) -> T::State {
        T::init_state(world)
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        T::get_state(components)
    }

    fn matches_component_set(
        _state: &T::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

// SAFETY: defers to soundness of `T: WorldQuery` impl
unsafe impl<T: QueryData> QueryData for Option<T> {
    const IS_READ_ONLY: bool = T::IS_READ_ONLY;
    type ReadOnly = Option<T::ReadOnly>;
    type Item<'w> = Option<T::Item<'w>>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item.map(T::shrink)
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w> {
        fetch
            .matches
            // SAFETY: The invariants are upheld by the caller.
            .then(|| unsafe { T::fetch(&mut fetch.fetch, entity, table_row) })
    }
}

/// SAFETY: [`OptionFetch`] is read only because `T` is read only
unsafe impl<T: ReadOnlyQueryData> ReadOnlyQueryData for Option<T> {}

/// Returns a bool that describes if an entity has the component `T`.
///
/// This can be used in a [`Query`](crate::system::Query) if you want to know whether or not entities
/// have the component `T`  but don't actually care about the component's value.
///
/// # Footguns
///
/// Note that a `Query<Has<T>>` will match all existing entities.
/// Beware! Even if it matches all entities, it doesn't mean that `query.get(entity)`
/// will always return `Ok(bool)`.
///
/// In the case of a non-existent entity, such as a despawned one, it will return `Err`.
/// A workaround is to replace `query.get(entity).unwrap()` by
/// `query.get(entity).unwrap_or_default()`.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::query::Has;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component)]
/// # struct IsHungry;
/// # #[derive(Component)]
/// # struct Name { name: &'static str };
/// #
/// fn food_entity_system(query: Query<(&Name, Has<IsHungry>) >) {
///     for (name, is_hungry) in &query {
///         if is_hungry{
///             println!("{} would like some food.", name.name);
///         } else {
///             println!("{} has had sufficient.", name.name);
///         }
///     }
/// }
/// # bevy_ecs::system::assert_is_system(food_entity_system);
/// ```
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::query::Has;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component)]
/// # struct Alpha{has_beta: bool};
/// # #[derive(Component)]
/// # struct Beta { has_alpha: bool };
/// #
/// // Unlike `Option<&T>`, `Has<T>` is compatible with `&mut T`
/// // as it does not actually access any data.
/// fn alphabet_entity_system(mut alphas: Query<(&mut Alpha, Has<Beta>)>, mut betas: Query<(&mut Beta, Has<Alpha>)>) {
///     for (mut alpha, has_beta) in alphas.iter_mut() {
///         alpha.has_beta = has_beta;
///     }
///     for (mut beta, has_alpha) in betas.iter_mut() {
///         beta.has_alpha = has_alpha;
///     }
/// }
/// # bevy_ecs::system::assert_is_system(alphabet_entity_system);
/// ```
pub struct Has<T>(PhantomData<T>);

impl<T> core::fmt::Debug for Has<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(f, "Has<{}>", core::any::type_name::<T>())
    }
}

/// SAFETY:
/// `update_component_access` and `update_archetype_component_access` do nothing.
/// This is sound because `fetch` does not access components.
unsafe impl<T: Component> WorldQuery for Has<T> {
    type Fetch<'w> = bool;
    type State = ComponentId;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
        fetch
    }

    #[inline]
    unsafe fn init_fetch<'w>(
        _world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
        false
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
        state: &Self::State,
        archetype: &'w Archetype,
        _table: &Table,
    ) {
        *fetch = archetype.contains(*state);
    }

    #[inline]
    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table) {
        *fetch = table.has_column(*state);
    }

    fn update_component_access(
        &component_id: &Self::State,
        access: &mut FilteredAccess<ComponentId>,
    ) {
        access.access_mut().add_archetypal(component_id);
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.register_component::<T>()
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        components.component_id::<T>()
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        // `Has<T>` always matches
        true
    }
}

/// SAFETY: `Self` is the same as `Self::ReadOnly`
unsafe impl<T: Component> QueryData for Has<T> {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = bool;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
        item
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
        *fetch
    }
}

/// SAFETY: [`Has`] is read only
unsafe impl<T: Component> ReadOnlyQueryData for Has<T> {}

/// The `AnyOf` query parameter fetches entities with any of the component types included in T.
///
/// `Query<AnyOf<(&A, &B, &mut C)>>` is equivalent to `Query<(Option<&A>, Option<&B>, Option<&mut C>), Or<(With<A>, With<B>, With<C>)>>`.
/// Each of the components in `T` is returned as an `Option`, as with `Option<A>` queries.
/// Entities are guaranteed to have at least one of the components in `T`.
pub struct AnyOf<T>(PhantomData<T>);

macro_rules! impl_tuple_query_data {
    ($(#[$meta:meta])* $(($name: ident, $state: ident)),*) => {
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such the lints below may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "The names of some variables are provided by the macro's caller, not by us."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        #[allow(
            clippy::unused_unit,
            reason = "Zero-length tuples will generate some function bodies equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
        )]
        $(#[$meta])*
        // SAFETY: defers to soundness `$name: WorldQuery` impl
        unsafe impl<$($name: QueryData),*> QueryData for ($($name,)*) {
            const IS_READ_ONLY: bool = true $(&& $name::IS_READ_ONLY)*;
            type ReadOnly = ($($name::ReadOnly,)*);
            type Item<'w> = ($($name::Item<'w>,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
                let ($($name,)*) = item;
                ($(
                    $name::shrink($name),
                )*)
            }

            #[inline(always)]
            unsafe fn fetch<'w>(
                fetch: &mut Self::Fetch<'w>,
                entity: Entity,
                table_row: TableRow
            ) -> Self::Item<'w> {
                let ($($name,)*) = fetch;
                // SAFETY: The invariants are upheld by the caller.
                ($(unsafe { $name::fetch($name, entity, table_row) },)*)
            }
        }

        $(#[$meta])*
        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyQueryData),*> ReadOnlyQueryData for ($($name,)*) {}

    };
}

macro_rules! impl_anytuple_fetch {
    ($(#[$meta:meta])* $(($name: ident, $state: ident)),*) => {
        $(#[$meta])*
        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such the lints below may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "The names of some variables are provided by the macro's caller, not by us."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        #[allow(
            clippy::unused_unit,
            reason = "Zero-length tuples will generate some function bodies equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
        )]
        /// SAFETY:
        /// `fetch` accesses are a subset of the subqueries' accesses
        /// This is sound because `update_component_access` and `update_archetype_component_access` adds accesses according to the implementations of all the subqueries.
        /// `update_component_access` replaces the filters with a disjunction where every element is a conjunction of the previous filters and the filters of one of the subqueries.
        /// This is sound because `matches_component_set` returns a disjunction of the results of the subqueries' implementations.
        unsafe impl<$($name: WorldQuery),*> WorldQuery for AnyOf<($($name,)*)> {
            type Fetch<'w> = ($(($name::Fetch<'w>, bool),)*);
            type State = ($($name::State,)*);

            fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
                let ($($name,)*) = fetch;
                ($(
                    ($name::shrink_fetch($name.0), $name.1),
                )*)
            }

            #[inline]
            unsafe fn init_fetch<'w>(_world: UnsafeWorldCell<'w>, state: &Self::State, _last_run: Tick, _this_run: Tick) -> Self::Fetch<'w> {
                let ($($name,)*) = state;
                // SAFETY: The invariants are upheld by the caller.
                ($(( unsafe { $name::init_fetch(_world, $name, _last_run, _this_run) }, false),)*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            #[inline]
            unsafe fn set_archetype<'w>(
                _fetch: &mut Self::Fetch<'w>,
                _state: &Self::State,
                _archetype: &'w Archetype,
                _table: &'w Table
            ) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $(
                    $name.1 = $name::matches_component_set($state, &|id| _archetype.contains(id));
                    if $name.1 {
                        // SAFETY: The invariants are upheld by the caller.
                        unsafe { $name::set_archetype(&mut $name.0, $state, _archetype, _table); }
                    }
                )*
            }

            #[inline]
            unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $(
                    $name.1 = $name::matches_component_set($state, &|id| _table.has_column(id));
                    if $name.1 {
                        // SAFETY: The invariants are required to be upheld by the caller.
                        unsafe { $name::set_table(&mut $name.0, $state, _table); }
                    }
                )*
            }

            fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
                // update the filters (Or<(With<$name>,)>)
                let ($($name,)*) = state;

                let mut _new_access = FilteredAccess::matches_nothing();

                $(
                    // Create an intermediate because `access`'s value needs to be preserved
                    // for the next query data, and `_new_access` has to be modified only by `append_or` to it,
                    // which only updates the `filter_sets`, not the `access`.
                    let mut intermediate = access.clone();
                    $name::update_component_access($name, &mut intermediate);
                    _new_access.append_or(&intermediate);
                )*

                // Of the accumulated `_new_access` we only care about the filter sets, not the access.
                access.filter_sets = _new_access.filter_sets;

                // For the access we instead delegate to a tuple of `Option`s.
                // This has essentially the same semantics of `AnyOf`, except that it doesn't
                // require at least one of them to be `Some`.
                // We however solve this by setting explicitly the `filter_sets` above.
                // Also note that Option<T> updates the `access` but not the `filter_sets`.
                <($(Option<$name>,)*)>::update_component_access(state, access);

            }
            fn init_state(world: &mut World) -> Self::State {
                ($($name::init_state(world),)*)
            }
            fn get_state(components: &Components) -> Option<Self::State> {
                Some(($($name::get_state(components)?,)*))
            }

            fn matches_component_set(_state: &Self::State, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($name,)*) = _state;
                false $(|| $name::matches_component_set($name, _set_contains_id))*
            }
        }

        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such the lints below may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "The names of some variables are provided by the macro's caller, not by us."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        #[allow(
            clippy::unused_unit,
            reason = "Zero-length tuples will generate some function bodies equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
        )]
        $(#[$meta])*
        // SAFETY: defers to soundness of `$name: WorldQuery` impl
        unsafe impl<$($name: QueryData),*> QueryData for AnyOf<($($name,)*)> {
            const IS_READ_ONLY: bool = true $(&& $name::IS_READ_ONLY)*;
            type ReadOnly = AnyOf<($($name::ReadOnly,)*)>;
            type Item<'w> = ($(Option<$name::Item<'w>>,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
                let ($($name,)*) = item;
                ($(
                    $name.map($name::shrink),
                )*)
            }

            #[inline(always)]
            unsafe fn fetch<'w>(
                _fetch: &mut Self::Fetch<'w>,
                _entity: Entity,
                _table_row: TableRow
            ) -> Self::Item<'w> {
                let ($($name,)*) = _fetch;
                ($(
                    // SAFETY: The invariants are required to be upheld by the caller.
                    $name.1.then(|| unsafe { $name::fetch(&mut $name.0, _entity, _table_row) }),
                )*)
            }
        }

        $(#[$meta])*
        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyQueryData),*> ReadOnlyQueryData for AnyOf<($($name,)*)> {}
    };
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_tuple_query_data,
    0,
    15,
    F,
    S
);
all_tuples!(
    #[doc(fake_variadic)]
    impl_anytuple_fetch,
    0,
    15,
    F,
    S
);

/// [`WorldQuery`] used to nullify queries by turning `Query<D>` into `Query<NopWorldQuery<D>>`
///
/// This will rarely be useful to consumers of `bevy_ecs`.
pub(crate) struct NopWorldQuery<D: QueryData>(PhantomData<D>);

/// SAFETY:
/// `update_component_access` and `update_archetype_component_access` do nothing.
/// This is sound because `fetch` does not access components.
unsafe impl<D: QueryData> WorldQuery for NopWorldQuery<D> {
    type Fetch<'w> = ();
    type State = D::State;

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(_: ()) {}

    #[inline(always)]
    unsafe fn init_fetch(
        _world: UnsafeWorldCell,
        _state: &D::State,
        _last_run: Tick,
        _this_run: Tick,
    ) {
    }

    const IS_DENSE: bool = D::IS_DENSE;

    #[inline(always)]
    unsafe fn set_archetype(
        _fetch: &mut (),
        _state: &D::State,
        _archetype: &Archetype,
        _tables: &Table,
    ) {
    }

    #[inline(always)]
    unsafe fn set_table<'w>(_fetch: &mut (), _state: &D::State, _table: &Table) {}

    fn update_component_access(_state: &D::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn init_state(world: &mut World) -> Self::State {
        D::init_state(world)
    }

    fn get_state(components: &Components) -> Option<Self::State> {
        D::get_state(components)
    }

    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        D::matches_component_set(state, set_contains_id)
    }
}

/// SAFETY: `Self::ReadOnly` is `Self`
unsafe impl<D: QueryData> QueryData for NopWorldQuery<D> {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'w> = ();

    fn shrink<'wlong: 'wshort, 'wshort>(_: ()) {}

    #[inline(always)]
    unsafe fn fetch<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }
}

/// SAFETY: `NopFetch` never accesses any data
unsafe impl<D: QueryData> ReadOnlyQueryData for NopWorldQuery<D> {}

/// SAFETY:
/// `update_component_access` and `update_archetype_component_access` do nothing.
/// This is sound because `fetch` does not access components.
unsafe impl<T: ?Sized> WorldQuery for PhantomData<T> {
    type Fetch<'a> = ();

    type State = ();

    fn shrink_fetch<'wlong: 'wshort, 'wshort>(_fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
    }

    unsafe fn init_fetch<'w>(
        _world: UnsafeWorldCell<'w>,
        _state: &Self::State,
        _last_run: Tick,
        _this_run: Tick,
    ) -> Self::Fetch<'w> {
    }

    // `PhantomData` does not match any components, so all components it matches
    // are stored in a Table (vacuous truth).
    const IS_DENSE: bool = true;

    unsafe fn set_archetype<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _table: &'w Table,
    ) {
    }

    unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
    }

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn init_state(_world: &mut World) -> Self::State {}

    fn get_state(_components: &Components) -> Option<Self::State> {
        Some(())
    }

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: `Self::ReadOnly` is `Self`
unsafe impl<T: ?Sized> QueryData for PhantomData<T> {
    const IS_READ_ONLY: bool = true;
    type ReadOnly = Self;
    type Item<'a> = ();

    fn shrink<'wlong: 'wshort, 'wshort>(_item: Self::Item<'wlong>) -> Self::Item<'wshort> {}

    unsafe fn fetch<'w>(
        _fetch: &mut Self::Fetch<'w>,
        _entity: Entity,
        _table_row: TableRow,
    ) -> Self::Item<'w> {
    }
}

/// SAFETY: `PhantomData` never accesses any world data.
unsafe impl<T: ?Sized> ReadOnlyQueryData for PhantomData<T> {}

/// A compile-time checked union of two different types that differs based on the
/// [`StorageType`] of a given component.
pub(super) union StorageSwitch<C: Component, T: Copy, S: Copy> {
    /// The table variant. Requires the component to be a table component.
    table: T,
    /// The sparse set variant. Requires the component to be a sparse set component.
    sparse_set: S,
    _marker: PhantomData<C>,
}

impl<C: Component, T: Copy, S: Copy> StorageSwitch<C, T, S> {
    /// Creates a new [`StorageSwitch`] using the given closures to initialize
    /// the variant corresponding to the component's [`StorageType`].
    pub fn new(table: impl FnOnce() -> T, sparse_set: impl FnOnce() -> S) -> Self {
        match C::STORAGE_TYPE {
            StorageType::Table => Self { table: table() },
            StorageType::SparseSet => Self {
                sparse_set: sparse_set(),
            },
        }
    }

    /// Creates a new [`StorageSwitch`] using a table variant.
    ///
    /// # Panics
    ///
    /// This will panic on debug builds if `C` is not a table component.
    ///
    /// # Safety
    ///
    /// `C` must be a table component.
    #[inline]
    pub unsafe fn set_table(&mut self, table: T) {
        match C::STORAGE_TYPE {
            StorageType::Table => self.table = table,
            _ => {
                #[cfg(debug_assertions)]
                unreachable!();
                #[cfg(not(debug_assertions))]
                core::hint::unreachable_unchecked()
            }
        }
    }

    /// Fetches the internal value from the variant that corresponds to the
    /// component's [`StorageType`].
    pub fn extract<R>(&self, table: impl FnOnce(T) -> R, sparse_set: impl FnOnce(S) -> R) -> R {
        match C::STORAGE_TYPE {
            StorageType::Table => table(
                // SAFETY: C::STORAGE_TYPE == StorageType::Table
                unsafe { self.table },
            ),
            StorageType::SparseSet => sparse_set(
                // SAFETY: C::STORAGE_TYPE == StorageType::SparseSet
                unsafe { self.sparse_set },
            ),
        }
    }
}

impl<C: Component, T: Copy, S: Copy> Clone for StorageSwitch<C, T, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C: Component, T: Copy, S: Copy> Copy for StorageSwitch<C, T, S> {}

#[cfg(test)]
mod tests {
    use bevy_ecs_macros::QueryData;

    use super::*;
    use crate::system::{assert_is_system, Query};

    #[derive(Component)]
    pub struct A;

    #[derive(Component)]
    pub struct B;

    // Tests that each variant of struct can be used as a `WorldQuery`.
    #[test]
    fn world_query_struct_variants() {
        #[derive(QueryData)]
        pub struct NamedQuery {
            id: Entity,
            a: &'static A,
        }

        #[derive(QueryData)]
        pub struct TupleQuery(&'static A, &'static B);

        #[derive(QueryData)]
        pub struct UnitQuery;

        fn my_system(_: Query<(NamedQuery, TupleQuery, UnitQuery)>) {}

        assert_is_system(my_system);
    }

    // Compile test for https://github.com/bevyengine/bevy/pull/8030.
    #[test]
    fn world_query_phantom_data() {
        #[derive(QueryData)]
        pub struct IgnoredQuery<Marker> {
            id: Entity,
            _marker: PhantomData<Marker>,
        }

        fn ignored_system(_: Query<IgnoredQuery<()>>) {}

        assert_is_system(ignored_system);
    }

    // Ensures that each field of a `WorldQuery` struct's read-only variant
    // has the same visibility as its corresponding mutable field.
    #[test]
    fn read_only_field_visibility() {
        mod private {
            use super::*;

            #[derive(QueryData)]
            #[query_data(mutable)]
            pub struct D {
                pub a: &'static mut A,
            }
        }

        let _ = private::DReadOnly { a: &A };

        fn my_system(query: Query<private::D>) {
            for q in &query {
                let _ = &q.a;
            }
        }

        assert_is_system(my_system);
    }

    // Ensures that metadata types generated by the WorldQuery macro
    // do not conflict with user-defined types.
    // Regression test for https://github.com/bevyengine/bevy/issues/8010.
    #[test]
    fn world_query_metadata_collision() {
        // The metadata types generated would be named `ClientState` and `ClientFetch`,
        // but they should rename themselves to avoid conflicts.
        #[derive(QueryData)]
        pub struct Client<S: ClientState> {
            pub state: &'static S,
            pub fetch: &'static ClientFetch,
        }

        pub trait ClientState: Component {}

        #[derive(Component)]
        pub struct ClientFetch;

        #[derive(Component)]
        pub struct C;

        impl ClientState for C {}

        fn client_system(_: Query<Client<C>>) {}

        assert_is_system(client_system);
    }
}
