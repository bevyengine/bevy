use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    change_detection::Ticks,
    component::{Component, ComponentId, ComponentStorage, ComponentTicks, StorageType},
    entity::Entity,
    query::{debug_checked_unreachable, Access, FilteredAccess},
    storage::{ComponentSparseSet, Table},
    world::{Mut, World},
};
use bevy_ecs_macros::all_tuples;
pub use bevy_ecs_macros::WorldQuery;
use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use std::{cell::UnsafeCell, marker::PhantomData};

/// Types that can be fetched from a [`World`] using a [`Query`].
///
/// There are many types that natively implement this trait:
///
/// - **Component references.**
///   Fetches a component by reference (immutably or mutably).
/// - **`WorldQuery` tuples.**
///   If every element of a tuple implements `WorldQuery`, then the tuple itself also implements the same trait.
///   This enables a single `Query` to access multiple components and filter over multiple conditions.
///   Due to the current lack of variadic generics in Rust, the trait has been implemented for tuples from 0 to 15 elements,
///   but nesting of tuples allows infinite `WorldQuery`s.
/// - **Component filters.**
///   [`With`] and [`Without`] filters can be applied to check if the queried entity contains or not a particular component.
/// - **Change detection filters.**
///   [`Added`] and [`Changed`] filters can be applied to detect component changes to an entity.
/// - **Filter disjunction operator.**
///   By default, tuples compose query filters in such a way that all conditions must be satisfied to generate a query item for a given entity.
///   Wrapping a tuple inside an [`Or`] operator will relax the requirement to just one condition.
/// - **[`Entity`].**
///   Gets the identifier of the queried entity.
/// - **[`Option`].**
///   By default, a world query only tests entities that have the matching component types.
///   Wrapping it into an `Option` will increase the query search space, and it will return `None` if an entity doesn't satisfy the `WorldQuery`.
/// - **[`AnyOf`].**
///   Equivalent to wrapping each world query inside it into an `Option`.
/// - **[`ChangeTrackers`].**
///   Similar to change detection filters but it is used as a query fetch parameter.
///   It exposes methods to check for changes to the wrapped component.
///
/// Implementing the trait manually can allow for a fundamentally new type of behaviour.
///
/// # Trait derivation
///
/// Query design can be easily structured by deriving `WorldQuery` for custom types.
/// Despite the added complexity, this approach has several advantages over using `WorldQuery` tuples.
/// The most relevant improvements are:
///
/// - Reusability across multiple systems.
/// - There is no need to destructure a tuple since all fields are named.
/// - Subqueries can be composed together to create a more complex query.
/// - Methods can be implemented for the query items.
/// - There is no hardcoded limit on the number of elements.
///
/// This trait can only be derived if each field of the struct also implements `WorldQuery`.
/// The derive macro only supports regular structs (structs with named fields).
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
///
/// #[derive(WorldQuery)]
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
/// Expanding the macro will declare three or six additional structs, depending on whether or not the struct is marked as mutable.
/// For a struct named `X`, the additional structs will be:
///
/// |Struct name|`mutable` only|Description|
/// |:---:|:---:|---|
/// |`XState`|---|Used as the [`State`] type for `X` and `XReadOnly`|
/// |`XItem`|---|The type of the query item for `X`|
/// |`XFetch`|---|Used as the [`Fetch`] type for `X`|
/// |`XReadOnlyItem`|✓|The type of the query item for `XReadOnly`|
/// |`XReadOnlyFetch`|✓|Used as the [`Fetch`] type for `XReadOnly`|
/// |`XReadOnly`|✓|[`ReadOnly`] variant of `X`|
///
/// ## Adding mutable references
///
/// Simply adding mutable references to a derived `WorldQuery` will result in a compilation error:
///
/// ```compile_fail
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::WorldQuery;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// #[derive(WorldQuery)]
/// struct CustomQuery {
///     component_a: &'static mut ComponentA,
/// }
/// ```
///
/// To grant mutable access to components, the struct must be marked with the `#[world_query(mutable)]` attribute.
/// This will also create three more structs that will be used for accessing the query immutably (see table above).
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::WorldQuery;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// #[derive(WorldQuery)]
/// #[world_query(mutable)]
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
/// # use bevy_ecs::query::WorldQuery;
/// #
/// #[derive(Component)]
/// struct Health(f32);
///
/// #[derive(Component)]
/// struct Buff(f32);
///
/// #[derive(WorldQuery)]
/// #[world_query(mutable)]
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
/// The `WorldQuery` derive macro does not automatically implement the traits of the struct to the query item types.
/// Something similar can be done by using the `#[world_query(derive(...))]` attribute.
/// This will apply the listed derivable traits to the query item structs.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::WorldQuery;
/// #
/// # #[derive(Component, Debug)]
/// # struct ComponentA;
/// #
/// #[derive(WorldQuery)]
/// #[world_query(mutable, derive(Debug))]
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
/// It is possible to use any `WorldQuery` as a field of another one.
/// This means that a `WorldQuery` can also be used as a subquery, potentially in multiple places.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::query::WorldQuery;
/// #
/// # #[derive(Component)]
/// # struct ComponentA;
/// # #[derive(Component)]
/// # struct ComponentB;
/// # #[derive(Component)]
/// # struct ComponentC;
/// #
/// #[derive(WorldQuery)]
/// struct SubQuery {
///     component_a: &'static ComponentA,
///     component_b: &'static ComponentB,
/// }
///
/// #[derive(WorldQuery)]
/// struct MyQuery {
///     subquery: SubQuery,
///     component_c: &'static ComponentC,
/// }
/// ```
///
/// ## Filters
///
/// Since the query filter type parameter is `WorldQuery`, it is also possible to use this macro to create filters.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::{query::WorldQuery, component::Component};
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
/// #[derive(WorldQuery)]
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
/// # Safety
///
/// Component access of `ROQueryFetch<Self>` must be a subset of `QueryFetch<Self>`
/// and `ROQueryFetch<Self>` must match exactly the same archetypes/tables as `QueryFetch<Self>`
///
/// Implementor must ensure that
/// [`update_component_access`] and [`update_archetype_component_access`]
/// exactly reflects the results of the following methods:
///
/// - [`matches_component_set`]
/// - [`fetch`]
///
/// [`Added`]: crate::query::Added
/// [`fetch`]: Self::fetch
/// [`Changed`]: crate::query::Changed
/// [`Fetch`]: crate::query::WorldQueryGats::Fetch
/// [`matches_component_set`]: Self::matches_component_set
/// [`Or`]: crate::query::Or
/// [`Query`]: crate::system::Query
/// [`ReadOnly`]: Self::ReadOnly
/// [`State`]: Self::State
/// [`update_archetype_component_access`]: Self::update_archetype_component_access
/// [`update_component_access`]: Self::update_component_access
/// [`With`]: crate::query::With
/// [`Without`]: crate::query::Without
pub unsafe trait WorldQuery: for<'w> WorldQueryGats<'w> {
    /// The read-only variant of this [`WorldQuery`], which satisfies the [`ReadOnlyWorldQuery`] trait.
    type ReadOnly: ReadOnlyWorldQuery<State = Self::State>;

    /// State used to construct a [`Self::Fetch`](crate::query::WorldQueryGats::Fetch). This will be cached inside [`QueryState`](crate::query::QueryState),
    /// so it is best to move as much data / computation here as possible to reduce the cost of
    /// constructing [`Self::Fetch`](crate::query::WorldQueryGats::Fetch).
    type State: Send + Sync + Sized;

    /// This function manually implements subtyping for the query items.
    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self>;

    /// Creates a new instance of this fetch.
    ///
    /// # Safety
    ///
    /// `state` must have been initialized (via [`WorldQuery::init_state`]) using the same `world` passed
    /// in to this function.
    unsafe fn init_fetch<'w>(
        world: &'w World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> <Self as WorldQueryGats<'w>>::Fetch;

    /// While this function can be called for any query, it is always safe to call if `Self: ReadOnlyWorldQuery` holds.
    ///
    /// # Safety
    /// While calling this method on its own cannot cause UB it is marked `unsafe` as the caller must ensure
    /// that the returned value is not used in any way that would cause two `QueryItem<Self>` for the same
    /// `archetype_index` or `table_row` to be alive at the same time.
    unsafe fn clone_fetch<'w>(
        fetch: &<Self as WorldQueryGats<'w>>::Fetch,
    ) -> <Self as WorldQueryGats<'w>>::Fetch;

    /// Returns true if (and only if) every table of every archetype matched by this fetch contains
    /// all of the matched components. This is used to select a more efficient "table iterator"
    /// for "dense" queries. If this returns true, [`WorldQuery::set_table`] must be used before
    /// [`WorldQuery::fetch`] can be called for iterators. If this returns false,
    /// [`WorldQuery::set_archetype`] must be used before [`WorldQuery::fetch`] can be called for
    /// iterators.
    const IS_DENSE: bool;

    /// Returns true if (and only if) this Fetch relies strictly on archetypes to limit which
    /// components are accessed by the Query.
    ///
    /// This enables optimizations for [`crate::query::QueryIter`] that rely on knowing exactly how
    /// many elements are being iterated (such as `Iterator::collect()`).
    const IS_ARCHETYPAL: bool;

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`WorldQuery`].
    ///
    /// # Safety
    ///
    /// `archetype` and `tables` must be from the [`World`] [`WorldQuery::init_state`] was called on. `state` must
    /// be the [`Self::State`] this was initialized with.
    unsafe fn set_archetype<'w>(
        fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        state: &Self::State,
        archetype: &'w Archetype,
        table: &'w Table,
    );

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`WorldQuery`].
    ///
    /// # Safety
    ///
    /// `table` must be from the [`World`] [`WorldQuery::init_state`] was called on. `state` must be the
    /// [`Self::State`] this was initialized with.
    unsafe fn set_table<'w>(
        fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        state: &Self::State,
        table: &'w Table,
    );

    /// Fetch [`Self::Item`](`WorldQueryGats::Item`) for either the given `entity` in the current [`Table`],
    /// or for the given `entity` in the current [`Archetype`]. This must always be called after
    /// [`WorldQuery::set_table`] with a `table_row` in the range of the current [`Table`] or after
    /// [`WorldQuery::set_archetype`]  with a `entity` in the current archetype.
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`WorldQuery::set_table`] or [`WorldQuery::set_archetype`]. `entity` and
    /// `table_row` must be in the range of the current table and archetype.
    unsafe fn fetch<'w>(
        fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        entity: Entity,
        table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item;

    /// # Safety
    ///
    /// Must always be called _after_ [`WorldQuery::set_table`] or [`WorldQuery::set_archetype`]. `entity` and
    /// `table_row` must be in the range of the current table and archetype.
    #[allow(unused_variables)]
    #[inline(always)]
    unsafe fn filter_fetch(
        fetch: &mut <Self as WorldQueryGats<'_>>::Fetch,
        entity: Entity,
        table_row: usize,
    ) -> bool {
        true
    }

    // This does not have a default body of `{}` because 99% of cases need to add accesses
    // and forgetting to do so would be unsound.
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>);
    // This does not have a default body of `{}` becaues 99% of cases need to add accesses
    // and forgetting to do so would be unsound.
    fn update_archetype_component_access(
        state: &Self::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    );

    fn init_state(world: &mut World) -> Self::State;
    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool;
}

/// A helper trait for [`WorldQuery`] that works around Rust's lack of Generic Associated Types.
///
/// **Note**: Consider using the type aliases [`QueryItem`] and [`QueryFetch`] when using `Item` or `Fetch`.
pub trait WorldQueryGats<'world> {
    type Item;
    type Fetch;
}

/// A world query that is read only.
///
/// # Safety
///
/// This must only be implemented for read-only [`WorldQuery`]'s.
pub unsafe trait ReadOnlyWorldQuery: WorldQuery<ReadOnly = Self> {}

/// The `Fetch` of a [`WorldQuery`], which is used to store state for each archetype/table.
pub type QueryFetch<'w, Q> = <Q as WorldQueryGats<'w>>::Fetch;
/// The item type returned when a [`WorldQuery`] is iterated over
pub type QueryItem<'w, Q> = <Q as WorldQueryGats<'w>>::Item;
/// The read-only `Fetch` of a [`WorldQuery`], which is used to store state for each archetype/table.
pub type ROQueryFetch<'w, Q> = QueryFetch<'w, <Q as WorldQuery>::ReadOnly>;
/// The read-only variant of the item type returned when a [`WorldQuery`] is iterated over immutably
pub type ROQueryItem<'w, Q> = QueryItem<'w, <Q as WorldQuery>::ReadOnly>;

/// SAFETY: no component or archetype access
unsafe impl WorldQuery for Entity {
    type ReadOnly = Self;
    type State = ();

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item
    }

    const IS_DENSE: bool = true;

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init_fetch<'w>(
        _world: &'w World,
        _state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> <Self as WorldQueryGats<'w>>::Fetch {
    }

    unsafe fn clone_fetch<'w>(
        _fetch: &<Self as WorldQueryGats<'w>>::Fetch,
    ) -> <Self as WorldQueryGats<'w>>::Fetch {
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _table: &Table,
    ) {
    }

    #[inline]
    unsafe fn set_table<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        _state: &Self::State,
        _table: &'w Table,
    ) {
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        entity: Entity,
        _table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
        entity
    }

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn update_archetype_component_access(
        _state: &Self::State,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn init_state(_world: &mut World) {}

    fn matches_component_set(
        _state: &Self::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

impl<'w> WorldQueryGats<'w> for Entity {
    type Fetch = ();
    type Item = Entity;
}

/// SAFETY: access is read only
unsafe impl ReadOnlyWorldQuery for Entity {}

#[doc(hidden)]
pub struct ReadFetch<'w, T> {
    // T::Storage = TableStorage
    table_components: Option<ThinSlicePtr<'w, UnsafeCell<T>>>,
    // T::Storage = SparseStorage
    sparse_set: Option<&'w ComponentSparseSet>,
}

/// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for &T {
    type ReadOnly = Self;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(item: &'wlong T) -> &'wshort T {
        item
    }

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init_fetch<'w>(
        world: &'w World,
        &component_id: &ComponentId,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> ReadFetch<'w, T> {
        ReadFetch {
            table_components: None,
            sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet).then(|| {
                world
                    .storages()
                    .sparse_sets
                    .get(component_id)
                    .unwrap_or_else(|| debug_checked_unreachable())
            }),
        }
    }

    unsafe fn clone_fetch<'w>(
        fetch: &<Self as WorldQueryGats<'w>>::Fetch,
    ) -> <Self as WorldQueryGats<'w>>::Fetch {
        ReadFetch {
            table_components: fetch.table_components,
            sparse_set: fetch.sparse_set,
        }
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut ReadFetch<'w, T>,
        component_id: &ComponentId,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if Self::IS_DENSE {
            Self::set_table(fetch, component_id, table);
        }
    }

    #[inline]
    unsafe fn set_table<'w>(
        fetch: &mut ReadFetch<'w, T>,
        &component_id: &ComponentId,
        table: &'w Table,
    ) {
        fetch.table_components = Some(
            table
                .get_column(component_id)
                .unwrap_or_else(|| debug_checked_unreachable())
                .get_data_slice()
                .into(),
        );
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        entity: Entity,
        table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => fetch
                .table_components
                .unwrap_or_else(|| debug_checked_unreachable())
                .get(table_row)
                .deref(),
            StorageType::SparseSet => fetch
                .sparse_set
                .unwrap_or_else(|| debug_checked_unreachable())
                .get(entity)
                .unwrap_or_else(|| debug_checked_unreachable())
                .deref(),
        }
    }

    fn update_component_access(
        &component_id: &ComponentId,
        access: &mut FilteredAccess<ComponentId>,
    ) {
        assert!(
            !access.access().has_write(component_id),
            "&{} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>(),
        );
        access.add_read(component_id);
    }

    fn update_archetype_component_access(
        &component_id: &ComponentId,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) = archetype.get_archetype_component_id(component_id) {
            access.add_read(archetype_component_id);
        }
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn matches_component_set(
        &state: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state)
    }
}

/// SAFETY: access is read only
unsafe impl<T: Component> ReadOnlyWorldQuery for &T {}

impl<'w, T: Component> WorldQueryGats<'w> for &T {
    type Fetch = ReadFetch<'w, T>;
    type Item = &'w T;
}

#[doc(hidden)]
pub struct WriteFetch<'w, T> {
    // T::Storage = TableStorage
    table_data: Option<(
        ThinSlicePtr<'w, UnsafeCell<T>>,
        ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>,
    )>,
    // T::Storage = SparseStorage
    sparse_set: Option<&'w ComponentSparseSet>,

    last_change_tick: u32,
    change_tick: u32,
}

/// SAFETY: access of `&T` is a subset of `&mut T`
unsafe impl<'__w, T: Component> WorldQuery for &'__w mut T {
    type ReadOnly = &'__w T;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(item: Mut<'wlong, T>) -> Mut<'wshort, T> {
        item
    }

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init_fetch<'w>(
        world: &'w World,
        &component_id: &ComponentId,
        last_change_tick: u32,
        change_tick: u32,
    ) -> WriteFetch<'w, T> {
        WriteFetch {
            table_data: None,
            sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet).then(|| {
                world
                    .storages()
                    .sparse_sets
                    .get(component_id)
                    .unwrap_or_else(|| debug_checked_unreachable())
            }),
            last_change_tick,
            change_tick,
        }
    }

    unsafe fn clone_fetch<'w>(
        fetch: &<Self as WorldQueryGats<'w>>::Fetch,
    ) -> <Self as WorldQueryGats<'w>>::Fetch {
        WriteFetch {
            table_data: fetch.table_data,
            sparse_set: fetch.sparse_set,
            last_change_tick: fetch.last_change_tick,
            change_tick: fetch.change_tick,
        }
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut WriteFetch<'w, T>,
        component_id: &ComponentId,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if Self::IS_DENSE {
            Self::set_table(fetch, component_id, table);
        }
    }

    #[inline]
    unsafe fn set_table<'w>(
        fetch: &mut WriteFetch<'w, T>,
        &component_id: &ComponentId,
        table: &'w Table,
    ) {
        let column = table
            .get_column(component_id)
            .unwrap_or_else(|| debug_checked_unreachable());
        fetch.table_data = Some((
            column.get_data_slice().into(),
            column.get_ticks_slice().into(),
        ));
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        entity: Entity,
        table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let (table_components, table_ticks) = fetch
                    .table_data
                    .unwrap_or_else(|| debug_checked_unreachable());
                Mut {
                    value: table_components.get(table_row).deref_mut(),
                    ticks: Ticks {
                        component_ticks: table_ticks.get(table_row).deref_mut(),
                        change_tick: fetch.change_tick,
                        last_change_tick: fetch.last_change_tick,
                    },
                }
            }
            StorageType::SparseSet => {
                let (component, component_ticks) = fetch
                    .sparse_set
                    .unwrap_or_else(|| debug_checked_unreachable())
                    .get_with_ticks(entity)
                    .unwrap_or_else(|| debug_checked_unreachable());
                Mut {
                    value: component.assert_unique().deref_mut(),
                    ticks: Ticks {
                        component_ticks: component_ticks.deref_mut(),
                        change_tick: fetch.change_tick,
                        last_change_tick: fetch.last_change_tick,
                    },
                }
            }
        }
    }

    fn update_component_access(
        &component_id: &ComponentId,
        access: &mut FilteredAccess<ComponentId>,
    ) {
        assert!(
            !access.access().has_read(component_id),
            "&mut {} conflicts with a previous access in this query. Mutable component access must be unique.",
                std::any::type_name::<T>(),
        );
        access.add_write(component_id);
    }

    fn update_archetype_component_access(
        &component_id: &ComponentId,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) = archetype.get_archetype_component_id(component_id) {
            access.add_write(archetype_component_id);
        }
    }

    fn init_state(world: &mut World) -> ComponentId {
        world.init_component::<T>()
    }

    fn matches_component_set(
        &state: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(state)
    }
}

impl<'w, T: Component> WorldQueryGats<'w> for &mut T {
    type Fetch = WriteFetch<'w, T>;
    type Item = Mut<'w, T>;
}

#[doc(hidden)]
pub struct OptionFetch<'w, T: WorldQuery> {
    fetch: <T as WorldQueryGats<'w>>::Fetch,
    matches: bool,
}

// SAFETY: defers to soundness of `T: WorldQuery` impl
unsafe impl<T: WorldQuery> WorldQuery for Option<T> {
    type ReadOnly = Option<T::ReadOnly>;
    type State = T::State;

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item.map(T::shrink)
    }

    const IS_DENSE: bool = T::IS_DENSE;

    const IS_ARCHETYPAL: bool = T::IS_ARCHETYPAL;

    unsafe fn init_fetch<'w>(
        world: &'w World,
        state: &T::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> OptionFetch<'w, T> {
        OptionFetch {
            fetch: T::init_fetch(world, state, last_change_tick, change_tick),
            matches: false,
        }
    }

    unsafe fn clone_fetch<'w>(
        fetch: &<Self as WorldQueryGats<'w>>::Fetch,
    ) -> <Self as WorldQueryGats<'w>>::Fetch {
        OptionFetch {
            fetch: T::clone_fetch(&fetch.fetch),
            matches: fetch.matches,
        }
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut OptionFetch<'w, T>,
        state: &T::State,
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        fetch.matches = T::matches_component_set(state, &|id| archetype.contains(id));
        if fetch.matches {
            T::set_archetype(&mut fetch.fetch, state, archetype, table);
        }
    }

    #[inline]
    unsafe fn set_table<'w>(fetch: &mut OptionFetch<'w, T>, state: &T::State, table: &'w Table) {
        fetch.matches = T::matches_component_set(state, &|id| table.has_column(id));
        if fetch.matches {
            T::set_table(&mut fetch.fetch, state, table);
        }
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        entity: Entity,
        table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
        fetch
            .matches
            .then(|| T::fetch(&mut fetch.fetch, entity, table_row))
    }

    fn update_component_access(state: &T::State, access: &mut FilteredAccess<ComponentId>) {
        // We don't want to add the `with`/`without` of `T` as `Option<T>` will match things regardless of
        // `T`'s filters. for example `Query<(Option<&U>, &mut V)>` will match every entity with a `V` component
        // regardless of whether it has a `U` component. If we dont do this the query will not conflict with
        // `Query<&mut V, Without<U>>` which would be unsound.
        let mut intermediate = access.clone();
        T::update_component_access(state, &mut intermediate);
        access.extend_access(&intermediate);
    }

    fn update_archetype_component_access(
        state: &T::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if T::matches_component_set(state, &|id| archetype.contains(id)) {
            T::update_archetype_component_access(state, archetype, access);
        }
    }

    fn init_state(world: &mut World) -> T::State {
        T::init_state(world)
    }

    fn matches_component_set(
        _state: &T::State,
        _set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        true
    }
}

/// SAFETY: [`OptionFetch`] is read only because `T` is read only
unsafe impl<T: ReadOnlyWorldQuery> ReadOnlyWorldQuery for Option<T> {}

impl<'w, T: WorldQuery> WorldQueryGats<'w> for Option<T> {
    type Fetch = OptionFetch<'w, T>;
    type Item = Option<QueryItem<'w, T>>;
}

/// [`WorldQuery`] that tracks changes and additions for component `T`.
///
/// Wraps a [`Component`] to track whether the component changed for the corresponding entities in
/// a query since the last time the system that includes these queries ran.
///
/// If you only care about entities that changed or that got added use the
/// [`Changed`](crate::query::Changed) and [`Added`](crate::query::Added) filters instead.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::query::ChangeTrackers;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component, Debug)]
/// # struct Name {};
/// # #[derive(Component)]
/// # struct Transform {};
/// #
/// fn print_moving_objects_system(query: Query<(&Name, ChangeTrackers<Transform>)>) {
///     for (name, tracker) in &query {
///         if tracker.is_changed() {
///             println!("Entity moved: {:?}", name);
///         } else {
///             println!("Entity stood still: {:?}", name);
///         }
///     }
/// }
/// # bevy_ecs::system::assert_is_system(print_moving_objects_system);
/// ```
pub struct ChangeTrackers<T: Component> {
    pub(crate) component_ticks: ComponentTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
    marker: PhantomData<T>,
}

impl<T: Component> Clone for ChangeTrackers<T> {
    fn clone(&self) -> Self {
        Self {
            component_ticks: self.component_ticks,
            last_change_tick: self.last_change_tick,
            change_tick: self.change_tick,
            marker: PhantomData,
        }
    }
}
impl<T: Component> Copy for ChangeTrackers<T> {}

impl<T: Component> std::fmt::Debug for ChangeTrackers<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChangeTrackers")
            .field("component_ticks", &self.component_ticks)
            .field("last_change_tick", &self.last_change_tick)
            .field("change_tick", &self.change_tick)
            .finish()
    }
}

impl<T: Component> ChangeTrackers<T> {
    /// Returns true if this component has been added since the last execution of this system.
    pub fn is_added(&self) -> bool {
        self.component_ticks
            .is_added(self.last_change_tick, self.change_tick)
    }

    /// Returns true if this component has been changed since the last execution of this system.
    pub fn is_changed(&self) -> bool {
        self.component_ticks
            .is_changed(self.last_change_tick, self.change_tick)
    }
}

#[doc(hidden)]
pub struct ChangeTrackersFetch<'w, T> {
    // T::Storage = TableStorage
    table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>>,
    // T::Storage = SparseStorage
    sparse_set: Option<&'w ComponentSparseSet>,

    marker: PhantomData<T>,
    last_change_tick: u32,
    change_tick: u32,
}

// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for ChangeTrackers<T> {
    type ReadOnly = Self;
    type State = ComponentId;

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item
    }

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init_fetch<'w>(
        world: &'w World,
        &component_id: &ComponentId,
        last_change_tick: u32,
        change_tick: u32,
    ) -> ChangeTrackersFetch<'w, T> {
        ChangeTrackersFetch {
            table_ticks: None,
            sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet).then(|| {
                world
                    .storages()
                    .sparse_sets
                    .get(component_id)
                    .unwrap_or_else(|| debug_checked_unreachable())
            }),
            marker: PhantomData,
            last_change_tick,
            change_tick,
        }
    }

    unsafe fn clone_fetch<'w>(
        fetch: &<Self as WorldQueryGats<'w>>::Fetch,
    ) -> <Self as WorldQueryGats<'w>>::Fetch {
        ChangeTrackersFetch {
            table_ticks: fetch.table_ticks,
            sparse_set: fetch.sparse_set,
            marker: fetch.marker,
            last_change_tick: fetch.last_change_tick,
            change_tick: fetch.change_tick,
        }
    }

    #[inline]
    unsafe fn set_archetype<'w>(
        fetch: &mut ChangeTrackersFetch<'w, T>,
        component_id: &ComponentId,
        _archetype: &'w Archetype,
        table: &'w Table,
    ) {
        if Self::IS_DENSE {
            Self::set_table(fetch, component_id, table);
        }
    }

    #[inline]
    unsafe fn set_table<'w>(
        fetch: &mut ChangeTrackersFetch<'w, T>,
        &id: &ComponentId,
        table: &'w Table,
    ) {
        fetch.table_ticks = Some(
            table
                .get_column(id)
                .unwrap_or_else(|| debug_checked_unreachable())
                .get_ticks_slice()
                .into(),
        );
    }

    #[inline(always)]
    unsafe fn fetch<'w>(
        fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        entity: Entity,
        table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => ChangeTrackers {
                component_ticks: {
                    let table_ticks = fetch
                        .table_ticks
                        .unwrap_or_else(|| debug_checked_unreachable());
                    table_ticks.get(table_row).read()
                },
                marker: PhantomData,
                last_change_tick: fetch.last_change_tick,
                change_tick: fetch.change_tick,
            },
            StorageType::SparseSet => ChangeTrackers {
                component_ticks: *fetch
                    .sparse_set
                    .unwrap_or_else(|| debug_checked_unreachable())
                    .get_ticks(entity)
                    .unwrap_or_else(|| debug_checked_unreachable())
                    .get(),
                marker: PhantomData,
                last_change_tick: fetch.last_change_tick,
                change_tick: fetch.change_tick,
            },
        }
    }

    fn update_component_access(&id: &ComponentId, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_write(id),
            "ChangeTrackers<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>()
        );
        access.add_read(id);
    }

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

    fn matches_component_set(
        &id: &ComponentId,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        set_contains_id(id)
    }
}

/// SAFETY: access is read only
unsafe impl<T: Component> ReadOnlyWorldQuery for ChangeTrackers<T> {}

impl<'w, T: Component> WorldQueryGats<'w> for ChangeTrackers<T> {
    type Fetch = ChangeTrackersFetch<'w, T>;
    type Item = ChangeTrackers<T>;
}

macro_rules! impl_tuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, $($name: WorldQueryGats<'w>),*> WorldQueryGats<'w> for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
            type Item = ($($name::Item,)*);
        }

        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        // SAFETY: defers to soundness `$name: WorldQuery` impl
        unsafe impl<$($name: WorldQuery),*> WorldQuery for ($($name,)*) {
            type ReadOnly = ($($name::ReadOnly,)*);
            type State = ($($name::State,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
                let ($($name,)*) = item;
                ($(
                    $name::shrink($name),
                )*)
            }

            #[allow(clippy::unused_unit)]
            unsafe fn init_fetch<'w>(_world: &'w World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> <Self as WorldQueryGats<'w>>::Fetch {
                let ($($name,)*) = state;
                ($($name::init_fetch(_world, $name, _last_change_tick, _change_tick),)*)
            }

            unsafe fn clone_fetch<'w>(
                fetch: &<Self as WorldQueryGats<'w>>::Fetch,
            ) -> <Self as WorldQueryGats<'w>>::Fetch {
                let ($($name,)*) = &fetch;
                ($($name::clone_fetch($name),)*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            const IS_ARCHETYPAL: bool = true $(&& $name::IS_ARCHETYPAL)*;

            #[inline]
            unsafe fn set_archetype<'w>(
                _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
                _state: &Self::State,
                _archetype: &'w Archetype,
                _table: &'w Table
            ) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $($name::set_archetype($name, $state, _archetype, _table);)*
            }

            #[inline]
            unsafe fn set_table<'w>(_fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, _state: &Self::State, _table: &'w Table) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $($name::set_table($name, $state, _table);)*
            }

            #[inline(always)]
            #[allow(clippy::unused_unit)]
            unsafe fn fetch<'w>(
                _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
                _entity: Entity,
                _table_row: usize
            ) -> <Self as WorldQueryGats<'w>>::Item {
                let ($($name,)*) = _fetch;
                ($($name::fetch($name, _entity, _table_row),)*)
            }

            #[inline(always)]
            unsafe fn filter_fetch<'w>(
                _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
                _entity: Entity,
                _table_row: usize
            ) -> bool {
                let ($($name,)*) = _fetch;
                true $(&& $name::filter_fetch($name, _entity, _table_row))*
            }

            fn update_component_access(state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = state;
                $($name::update_component_access($name, _access);)*
            }

            fn update_archetype_component_access(state: &Self::State, _archetype: &Archetype, _access: &mut Access<ArchetypeComponentId>) {
                let ($($name,)*) = state;
                $($name::update_archetype_component_access($name, _archetype, _access);)*
            }


            fn init_state(_world: &mut World) -> Self::State {
                ($($name::init_state(_world),)*)
            }

            fn matches_component_set(state: &Self::State, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($name,)*) = state;
                true $(&& $name::matches_component_set($name, _set_contains_id))*
            }
        }

        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyWorldQuery),*> ReadOnlyWorldQuery for ($($name,)*) {}

    };
}

/// The `AnyOf` query parameter fetches entities with any of the component types included in T.
///
/// `Query<AnyOf<(&A, &B, &mut C)>>` is equivalent to `Query<(Option<&A>, Option<&B>, Option<&mut C>), Or<(With<A>, With<B>, With<C>)>>`.
/// Each of the components in `T` is returned as an `Option`, as with `Option<A>` queries.
/// Entities are guaranteed to have at least one of the components in `T`.
pub struct AnyOf<T>(PhantomData<T>);

macro_rules! impl_anytuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, $($name: WorldQueryGats<'w>),*> WorldQueryGats<'w> for AnyOf<($($name,)*)> {
            type Fetch = ($(($name::Fetch, bool),)*);
            type Item = ($(Option<$name::Item>,)*);
        }

        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        // SAFETY: defers to soundness of `$name: WorldQuery` impl
        unsafe impl<$($name: WorldQuery),*> WorldQuery for AnyOf<($($name,)*)> {
            type ReadOnly = AnyOf<($($name::ReadOnly,)*)>;
            type State = ($($name::State,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
                let ($($name,)*) = item;
                ($(
                    $name.map($name::shrink),
                )*)
            }

            #[allow(clippy::unused_unit)]
            unsafe fn init_fetch<'w>(_world: &'w World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> <Self as WorldQueryGats<'w>>::Fetch {
                let ($($name,)*) = state;
                ($(($name::init_fetch(_world, $name, _last_change_tick, _change_tick), false),)*)
            }

            unsafe fn clone_fetch<'w>(
                fetch: &<Self as WorldQueryGats<'w>>::Fetch,
            ) -> <Self as WorldQueryGats<'w>>::Fetch {
                let ($($name,)*) = &fetch;
                ($(($name::clone_fetch(& $name.0), $name.1),)*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            const IS_ARCHETYPAL: bool = true $(&& $name::IS_ARCHETYPAL)*;

            #[inline]
            unsafe fn set_archetype<'w>(
                _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
                _state: &Self::State,
                _archetype: &'w Archetype,
                _table: &'w Table
            ) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $(
                    $name.1 = $name::matches_component_set($state, &|id| _archetype.contains(id));
                    if $name.1 {
                        $name::set_archetype(&mut $name.0, $state, _archetype, _table);
                    }
                )*
            }

            #[inline]
            unsafe fn set_table<'w>(_fetch: &mut <Self as WorldQueryGats<'w>>::Fetch, _state: &Self::State, _table: &'w Table) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $(
                    $name.1 = $name::matches_component_set($state, &|id| _table.has_column(id));
                    if $name.1 {
                        $name::set_table(&mut $name.0, $state, _table);
                    }
                )*
            }

            #[inline(always)]
            #[allow(clippy::unused_unit)]
            unsafe fn fetch<'w>(
                _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
                _entity: Entity,
                _table_row: usize
            ) -> <Self as WorldQueryGats<'w>>::Item {
                let ($($name,)*) = _fetch;
                ($(
                    $name.1.then(|| $name::fetch(&mut $name.0, _entity, _table_row)),
                )*)
            }

            fn update_component_access(state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = state;

                // We do not unconditionally add `$name`'s `with`/`without` accesses to `_access`
                // as this would be unsound. For example the following two queries should conflict:
                // - Query<(AnyOf<(&A, ())>, &mut B)>
                // - Query<&mut B, Without<A>>
                //
                // If we were to unconditionally add `$name`'s `with`/`without` accesses then `AnyOf<(&A, ())>`
                // would have a `With<A>` access which is incorrect as this `WorldQuery` will match entities that
                // do not have the `A` component. This is the same logic as the `Or<...>: WorldQuery` impl.
                //
                // The correct thing to do here is to only add a `with`/`without` access to `_access` if all
                // `$name` params have that `with`/`without` access. More jargony put- we add the intersection
                // of all `with`/`without` accesses of the `$name` params to `_access`.
                let mut _intersected_access = _access.clone();
                let mut _not_first = false;
                $(
                    if _not_first {
                        let mut intermediate = _access.clone();
                        $name::update_component_access($name, &mut intermediate);
                        _intersected_access.extend_intersect_filter(&intermediate);
                        _intersected_access.extend_access(&intermediate);
                    } else {

                        $name::update_component_access($name, &mut _intersected_access);
                        _not_first = true;
                    }
                )*

                *_access = _intersected_access;
            }

            fn update_archetype_component_access(state: &Self::State, _archetype: &Archetype, _access: &mut Access<ArchetypeComponentId>) {
                let ($($name,)*) = state;
                $(
                    if $name::matches_component_set($name, &|id| _archetype.contains(id)) {
                        $name::update_archetype_component_access($name, _archetype, _access);
                    }
                )*
            }

            fn init_state(_world: &mut World) -> Self::State {
                ($($name::init_state(_world),)*)
            }

            fn matches_component_set(_state: &Self::State, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($name,)*) = _state;
                false $(|| $name::matches_component_set($name, _set_contains_id))*
            }
        }

        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyWorldQuery),*> ReadOnlyWorldQuery for AnyOf<($($name,)*)> {}

    };
}

all_tuples!(impl_tuple_fetch, 0, 15, F, S);
all_tuples!(impl_anytuple_fetch, 0, 15, F, S);

/// [`WorldQuery`] used to nullify queries by turning `Query<Q>` into `Query<NopWorldQuery<Q>>`
///
/// This will rarely be useful to consumers of `bevy_ecs`.
pub struct NopWorldQuery<Q: WorldQuery>(PhantomData<Q>);

/// SAFETY: `Self::ReadOnly` is `Self`
unsafe impl<Q: WorldQuery> WorldQuery for NopWorldQuery<Q> {
    type ReadOnly = Self;
    type State = Q::State;

    fn shrink<'wlong: 'wshort, 'wshort>(_: ()) {}

    const IS_DENSE: bool = Q::IS_DENSE;

    const IS_ARCHETYPAL: bool = true;

    #[inline(always)]
    unsafe fn init_fetch(
        _world: &World,
        _state: &Q::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) {
    }

    unsafe fn clone_fetch<'w>(
        _fetch: &<Self as WorldQueryGats<'w>>::Fetch,
    ) -> <Self as WorldQueryGats<'w>>::Fetch {
    }

    #[inline(always)]
    unsafe fn set_archetype(
        _fetch: &mut (),
        _state: &Q::State,
        _archetype: &Archetype,
        _tables: &Table,
    ) {
    }

    #[inline(always)]
    unsafe fn set_table<'w>(_fetch: &mut (), _state: &Q::State, _table: &Table) {}

    #[inline(always)]
    unsafe fn fetch<'w>(
        _fetch: &mut <Self as WorldQueryGats<'w>>::Fetch,
        _entity: Entity,
        _table_row: usize,
    ) -> <Self as WorldQueryGats<'w>>::Item {
    }

    fn update_component_access(_state: &Q::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn update_archetype_component_access(
        _state: &Q::State,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn init_state(world: &mut World) -> Self::State {
        Q::init_state(world)
    }

    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool {
        Q::matches_component_set(state, set_contains_id)
    }
}

impl<'a, Q: WorldQuery> WorldQueryGats<'a> for NopWorldQuery<Q> {
    type Fetch = ();
    type Item = ();
}
/// SAFETY: `NopFetch` never accesses any data
unsafe impl<Q: WorldQuery> ReadOnlyWorldQuery for NopWorldQuery<Q> {}
