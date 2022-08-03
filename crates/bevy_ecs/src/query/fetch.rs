use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    change_detection::Ticks,
    component::{Component, ComponentId, ComponentStorage, ComponentTicks, StorageType},
    entity::Entity,
    query::{debug_checked_unreachable, Access, FilteredAccess},
    storage::{ComponentSparseSet, Table, Tables},
    world::{Mut, World},
};
use bevy_ecs_macros::all_tuples;
pub use bevy_ecs_macros::WorldQuery;
use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use std::{cell::UnsafeCell, marker::PhantomData};

/// Types that can be queried from a [`World`].
///
/// Notable types that implement this trait are `&T` and `&mut T` where `T` implements [`Component`],
/// allowing you to query for components immutably and mutably accordingly.
///
/// See [`Query`](crate::system::Query) for a primer on queries.
///
/// # Basic [`WorldQuery`]'s
///
/// Here is a small list of the most important world queries to know about where `C` stands for a
/// [`Component`] and `WQ` stands for a [`WorldQuery`]:
/// - `&C`: Queries immutably for the component `C`
/// - `&mut C`: Queries mutably for the component `C`
/// - `Option<WQ>`: Queries the inner [`WorldQuery`] `WQ` but instead of discarding the entity if the world
///     query fails it returns [`None`]. See [`Query`](crate::system::Query).
/// - `(WQ1, WQ2, ...)`: Queries all contained world queries allowing to query for more than one thing.
///     This is the `And` operator for filters. See [`Or`].
/// - `ChangeTrackers<C>`: See the docs of [`ChangeTrackers`].
/// - [`Entity`]: Using the entity type as a world query will grant access to the entity that is
///     being queried for. See [`Entity`].
///
/// Bevy also offers a few filters like [`Added`](crate::query::Added), [`Changed`](crate::query::Changed),
/// [`With`](crate::query::With), [`Without`](crate::query::Without) and [`Or`].
/// For more information on these consult the item's corresponding documentation.
///
/// [`Or`]: crate::query::Or
///
/// # Derive
///
/// This trait can be derived with the [`derive@super::WorldQuery`] macro.
///
/// You may want to implement a custom query with the derive macro for the following reasons:
/// - Named structs can be clearer and easier to use than complex query tuples. Access via struct
///   fields is more convenient than destructuring tuples or accessing them via `q.0, q.1, ...`
///   pattern and saves a lot of maintenance burden when adding or removing components.
/// - Nested queries enable the composition pattern and makes query types easier to re-use.
/// - You can bypass the limit of 15 components that exists for query tuples.
///
/// Implementing the trait manually can allow for a fundamentally new type of behaviour.
///
/// The derive macro implements [`WorldQuery`] for your type and declares an additional struct
/// which will be used as an item for query iterators. The implementation also generates two other
/// structs that implement [`Fetch`] and [`FetchState`] and are used as [`WorldQuery::Fetch`](WorldQueryGats::Fetch) and
/// [`WorldQuery::State`] associated types respectively.
///
/// The derive macro requires every struct field to implement the `WorldQuery` trait.
///
/// **Note:** currently, the macro only supports named structs.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
///
/// #[derive(Component)]
/// struct Foo;
/// #[derive(Component)]
/// struct Bar;
///
/// #[derive(WorldQuery)]
/// struct MyQuery {
///     entity: Entity,
///     // We must explicitly list out all lifetimes, as we are defining a struct
///     foo: &'static Foo,
///     bar: Option<&'static Bar>,
/// }
///
/// fn my_system(query: Query<MyQuery>) {
///     for q in &query {
///         // Note the type of the returned item.
///         let q: MyQueryItem<'_> = q;
///         q.foo;
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// ## Mutable queries
///
/// All queries that are derived with the `WorldQuery` macro provide only an immutable access by default.
/// If you need a mutable access to components, you can mark a struct with the `mutable` attribute.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
///
/// #[derive(Component)]
/// struct Health(f32);
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
/// // This implementation is only available when iterating with `iter_mut`.
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
/// // If you want to use it with `iter`, you'll need to write an additional implementation.
/// impl<'w> HealthQueryReadOnlyItem<'w> {
///     fn total(&self) -> f32 {
///         self.health.0 + self.buff.map_or(0.0, |Buff(buff)| *buff)
///     }
/// }
///
/// fn my_system(mut health_query: Query<HealthQuery>) {
///     // Iterator's item is `HealthQueryReadOnlyItem`.
///     for health in &health_query {
///         println!("Total: {}", health.total());
///     }
///     // Iterator's item is `HealthQueryItem`.
///     for mut health in &mut health_query {
///         health.damage(1.0);
///         println!("Total (mut): {}", health.total());
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// Mutable queries will also have a read only version derived:
/// ```rust
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
///
/// #[derive(Component)]
/// pub struct MyComponent;
///
/// #[derive(WorldQuery)]
/// #[world_query(mutable)]
/// pub struct Foo {
///     my_component_yay: &'static mut MyComponent,
/// }
///
/// fn my_system(mut my_query: Query<(FooReadOnly, FooReadOnly)>) {
///     for (i1, i2) in &mut my_query {
///         let _: FooReadOnlyItem<'_> = i1;
///         let _: FooReadOnlyItem<'_> = i2;
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// **Note:** if you omit the `mutable` attribute for a query that doesn't implement
/// [`ReadOnlyWorldQuery`], compilation will fail. We insert static checks as in the example above for
/// every query component and a nested query.
/// (The checks neither affect the runtime, nor pollute your local namespace.)
///
/// ```compile_fail
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
///
/// #[derive(Component)]
/// struct Foo;
/// #[derive(Component)]
/// struct Bar;
///
/// #[derive(WorldQuery)]
/// struct FooQuery {
///     foo: &'static Foo,
///     bar_query: BarQuery,
/// }
///
/// #[derive(WorldQuery)]
/// #[world_query(mutable)]
/// struct BarQuery {
///     bar: &'static mut Bar,
/// }
/// ```
///
/// ## Derives for items
///
/// If you want query items to have derivable traits, you can pass them with using
/// the `world_query(derive)` attribute. When the `WorldQuery` macro generates the structs
/// for query items, it doesn't automatically inherit derives of a query itself. Since derive macros
/// can't access information about other derives, they need to be passed manually with the
/// `world_query(derive)` attribute.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
///
/// #[derive(Component, Debug)]
/// struct Foo;
///
/// #[derive(WorldQuery)]
/// #[world_query(mutable, derive(Debug))]
/// struct FooQuery {
///     foo: &'static Foo,
/// }
///
/// fn assert_debug<T: std::fmt::Debug>() {}
///
/// assert_debug::<FooQueryItem>();
/// assert_debug::<FooQueryReadOnlyItem>();
/// ```
///
/// ## Nested queries
///
/// Using nested queries enable the composition pattern, which makes it possible to re-use other
/// query types. All types that implement [`WorldQuery`] (including the ones that use this derive
/// macro) are supported.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
///
/// #[derive(Component)]
/// struct Foo;
/// #[derive(Component)]
/// struct Bar;
/// #[derive(Component)]
/// struct OptionalFoo;
/// #[derive(Component)]
/// struct OptionalBar;
///
/// #[derive(WorldQuery)]
/// struct MyQuery {
///     foo: FooQuery,
///     bar: (&'static Bar, Option<&'static OptionalBar>)
/// }
///
/// #[derive(WorldQuery)]
/// struct FooQuery {
///     foo: &'static Foo,
///     optional_foo: Option<&'static OptionalFoo>,
/// }
///
/// // You can also compose derived queries with regular ones in tuples.
/// fn my_system(query: Query<(&Foo, MyQuery, FooQuery)>) {
///     for (foo, my_query, foo_query) in &query {
///         foo; my_query; foo_query;
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// ## Ignored fields
///
/// The macro also supports `ignore` attribute for struct members. Fields marked with this attribute
/// must implement the `Default` trait.
///
/// This example demonstrates a query that would iterate over every entity.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::query::WorldQuery;
///
/// #[derive(WorldQuery, Debug)]
/// struct EmptyQuery {
///     empty: (),
/// }
///
/// fn my_system(query: Query<EmptyQuery>) {
///     for _ in &query {}
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// ## Filters
///
/// Using [`derive@super::WorldQuery`] macro we can create our own query filters.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// use bevy_ecs::{query::WorldQuery, component::Component};
///
/// #[derive(Component)]
/// struct Foo;
/// #[derive(Component)]
/// struct Bar;
/// #[derive(Component)]
/// struct Baz;
/// #[derive(Component)]
/// struct Qux;
///
/// #[derive(WorldQuery)]
/// struct MyFilter<T: Component, P: Component> {
///     _foo: With<Foo>,
///     _bar: With<Bar>,
///     _or: Or<(With<Baz>, Changed<Foo>, Added<Bar>)>,
///     _generic_tuple: (With<T>, Without<P>),
/// }
///
/// fn my_system(query: Query<Entity, MyFilter<Foo, Qux>>) {
///     for _ in &query {}
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
/// # Safety
///
/// component access of `ROQueryFetch<Self>` must be a subset of `QueryFetch<Self>`
/// and `ROQueryFetch<Self>` must match exactly the same archetypes/tables as `QueryFetch<Self>`
pub unsafe trait WorldQuery: for<'w> WorldQueryGats<'w, _State = Self::State> {
    type ReadOnly: ReadOnlyWorldQuery<State = Self::State>;
    type State: FetchState;

    /// This function manually implements variance for the query items.
    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self>;
}

/// A world query that is read only.
///
/// # Safety
///
/// This must only be implemented for read-only [`WorldQuery`]'s.
pub unsafe trait ReadOnlyWorldQuery: WorldQuery<ReadOnly = Self> {}

/// The [`Fetch`] of a [`WorldQuery`], which declares which data it needs access to
pub type QueryFetch<'w, Q> = <Q as WorldQueryGats<'w>>::Fetch;
/// The item type returned when a [`WorldQuery`] is iterated over
pub type QueryItem<'w, Q> = <<Q as WorldQueryGats<'w>>::Fetch as Fetch<'w>>::Item;
/// The read-only [`Fetch`] of a [`WorldQuery`], which declares which data it needs access to when accessed immutably
pub type ROQueryFetch<'w, Q> = QueryFetch<'w, <Q as WorldQuery>::ReadOnly>;
/// The read-only variant of the item type returned when a [`WorldQuery`] is iterated over immutably
pub type ROQueryItem<'w, Q> = QueryItem<'w, <Q as WorldQuery>::ReadOnly>;

/// A helper trait for [`WorldQuery`] that works around Rust's lack of Generic Associated Types
pub trait WorldQueryGats<'world> {
    type Fetch: Fetch<'world, State = Self::_State>;
    type _State: FetchState;
}

/// Types that implement this trait are responsible for fetching query items from tables or
/// archetypes.
///
/// Every type that implements [`WorldQuery`] have their associated [`WorldQuery::Fetch`](WorldQueryGats::Fetch)  and
/// [`WorldQuery::State`] types that are essential for fetching component data.
///
/// # Safety
///
/// Implementor must ensure that [`Fetch::update_component_access`] and
/// [`Fetch::update_archetype_component_access`] exactly reflects the results of
/// [`FetchState::matches_component_set`], [`Fetch::archetype_fetch`], and
/// [`Fetch::table_fetch`].
pub unsafe trait Fetch<'world>: Sized {
    type Item;
    type State: FetchState;

    /// Creates a new instance of this fetch.
    ///
    /// # Safety
    ///
    /// `state` must have been initialized (via [`FetchState::init`]) using the same `world` passed
    /// in to this function.
    unsafe fn init(
        world: &'world World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self;

    /// Returns true if (and only if) every table of every archetype matched by this fetch contains
    /// all of the matched components. This is used to select a more efficient "table iterator"
    /// for "dense" queries. If this returns true, [`Fetch::set_table`] and [`Fetch::table_fetch`]
    /// will be called for iterators. If this returns false, [`Fetch::set_archetype`] and
    /// [`Fetch::archetype_fetch`] will be called for iterators.
    const IS_DENSE: bool;

    /// Returns true if (and only if) this Fetch relies strictly on archetypes to limit which
    /// components are accessed by the Query.
    ///
    /// This enables optimizations for [`crate::query::QueryIter`] that rely on knowing exactly how
    /// many elements are being iterated (such as `Iterator::collect()`).
    const IS_ARCHETYPAL: bool;

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `archetype` and `tables` must be from the [`World`] [`Fetch::init`] was called on. `state` must
    /// be the [`Self::State`] this was initialized with.
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &'world Archetype,
        tables: &'world Tables,
    );

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `table` must be from the [`World`] [`Fetch::init`] was called on. `state` must be the
    /// [`Self::State`] this was initialized with.
    unsafe fn set_table(&mut self, state: &Self::State, table: &'world Table);

    /// Fetch [`Self::Item`] for the given `archetype_index` in the current [`Archetype`]. This must
    /// always be called after [`Fetch::set_archetype`] with an `archetype_index` in the range of
    /// the current [`Archetype`]
    ///
    /// # Safety
    /// Must always be called _after_ [`Fetch::set_archetype`]. `archetype_index` must be in the range
    /// of the current archetype
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item;

    /// Fetch [`Self::Item`] for the given `table_row` in the current [`Table`]. This must always be
    /// called after [`Fetch::set_table`] with a `table_row` in the range of the current [`Table`]
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`Fetch::set_table`]. `table_row` must be in the range of the
    /// current table
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item;

    /// # Safety
    ///
    /// Must always be called _after_ [`Fetch::set_archetype`]. `archetype_index` must be in the range
    /// of the current archetype.
    #[allow(unused_variables)]
    #[inline]
    unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool {
        true
    }

    /// # Safety
    ///
    /// Must always be called _after_ [`Fetch::set_table`]. `table_row` must be in the range of the
    /// current table.
    #[allow(unused_variables)]
    #[inline]
    unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool {
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
}

/// State used to construct a Fetch. This will be cached inside [`QueryState`](crate::query::QueryState),
///  so it is best to move as much data / computation here as possible to reduce the cost of
/// constructing Fetch.
pub trait FetchState: Send + Sync + Sized {
    fn init(world: &mut World) -> Self;
    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool;
}

/// SAFETY: no component or archetype access
unsafe impl WorldQuery for Entity {
    type ReadOnly = Self;
    type State = EntityState;

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item
    }
}

/// The [`Fetch`] of [`Entity`].
#[doc(hidden)]
#[derive(Clone)]
pub struct EntityFetch<'w> {
    entities: Option<ThinSlicePtr<'w, Entity>>,
}

/// SAFETY: access is read only
unsafe impl ReadOnlyWorldQuery for Entity {}

/// The [`FetchState`] of [`Entity`].
#[doc(hidden)]
pub struct EntityState;

impl FetchState for EntityState {
    fn init(_world: &mut World) -> Self {
        Self
    }

    #[inline]
    fn matches_component_set(&self, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        true
    }
}

impl<'w> WorldQueryGats<'w> for Entity {
    type Fetch = EntityFetch<'w>;
    type _State = EntityState;
}

/// SAFETY: no component or archetype access
unsafe impl<'w> Fetch<'w> for EntityFetch<'w> {
    type Item = Entity;
    type State = EntityState;

    const IS_DENSE: bool = true;

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init(
        _world: &'w World,
        _state: &EntityState,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> EntityFetch<'w> {
        EntityFetch { entities: None }
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        archetype: &'w Archetype,
        _tables: &Tables,
    ) {
        self.entities = Some(archetype.entities().into());
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, table: &'w Table) {
        self.entities = Some(table.entities().into());
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let entities = self.entities.unwrap_or_else(|| debug_checked_unreachable());
        *entities.get(table_row)
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        let entities = self.entities.unwrap_or_else(|| debug_checked_unreachable());
        *entities.get(archetype_index)
    }

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn update_archetype_component_access(
        _state: &Self::State,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }
}

/// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for &T {
    type ReadOnly = Self;
    type State = ComponentIdState<T>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item
    }
}

/// The [`FetchState`] of `&T`.
#[doc(hidden)]
pub struct ComponentIdState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<T: Component> FetchState for ComponentIdState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        ComponentIdState {
            component_id,
            marker: PhantomData,
        }
    }

    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        set_contains_id(self.component_id)
    }
}

/// The [`Fetch`] of `&T`.
#[doc(hidden)]
pub struct ReadFetch<'w, T> {
    // T::Storage = TableStorage
    table_components: Option<ThinSlicePtr<'w, UnsafeCell<T>>>,
    entity_table_rows: Option<ThinSlicePtr<'w, usize>>,
    // T::Storage = SparseStorage
    entities: Option<ThinSlicePtr<'w, Entity>>,
    sparse_set: Option<&'w ComponentSparseSet>,
}

impl<T> Clone for ReadFetch<'_, T> {
    fn clone(&self) -> Self {
        Self {
            table_components: self.table_components,
            entity_table_rows: self.entity_table_rows,
            entities: self.entities,
            sparse_set: self.sparse_set,
        }
    }
}

/// SAFETY: access is read only
unsafe impl<T: Component> ReadOnlyWorldQuery for &T {}

impl<'w, T: Component> WorldQueryGats<'w> for &T {
    type Fetch = ReadFetch<'w, T>;
    type _State = ComponentIdState<T>;
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// read
unsafe impl<'w, T: Component> Fetch<'w> for ReadFetch<'w, T> {
    type Item = &'w T;
    type State = ComponentIdState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init(
        world: &'w World,
        state: &ComponentIdState<T>,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> ReadFetch<'w, T> {
        ReadFetch {
            table_components: None,
            entity_table_rows: None,
            entities: None,
            sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet).then(|| {
                world
                    .storages()
                    .sparse_sets
                    .get(state.component_id)
                    .unwrap()
            }),
        }
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &'w Archetype,
        tables: &'w Tables,
    ) {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                self.entity_table_rows = Some(archetype.entity_table_rows().into());
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = Some(column.get_data_slice().into());
            }
            StorageType::SparseSet => self.entities = Some(archetype.entities().into()),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        self.table_components = Some(
            table
                .get_column(state.component_id)
                .unwrap()
                .get_data_slice()
                .into(),
        );
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let (entity_table_rows, table_components) = self
                    .entity_table_rows
                    .zip(self.table_components)
                    .unwrap_or_else(|| debug_checked_unreachable());
                let table_row = *entity_table_rows.get(archetype_index);
                table_components.get(table_row).deref()
            }
            StorageType::SparseSet => {
                let (entities, sparse_set) = self
                    .entities
                    .zip(self.sparse_set)
                    .unwrap_or_else(|| debug_checked_unreachable());
                let entity = *entities.get(archetype_index);
                sparse_set
                    .get(entity)
                    .unwrap_or_else(|| debug_checked_unreachable())
                    .deref::<T>()
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let components = self
            .table_components
            .unwrap_or_else(|| debug_checked_unreachable());
        components.get(table_row).deref()
    }

    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_write(state.component_id),
            "&{} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>(),
        );
        access.add_read(state.component_id);
    }

    fn update_archetype_component_access(
        state: &Self::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(state.component_id)
        {
            access.add_read(archetype_component_id);
        }
    }
}

/// SAFETY: access of `&T` is a subset of `&mut T`
unsafe impl<'w, T: Component> WorldQuery for &'w mut T {
    type ReadOnly = &'w T;
    type State = ComponentIdState<T>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item
    }
}

/// The [`Fetch`] of `&mut T`.
#[doc(hidden)]
pub struct WriteFetch<'w, T> {
    // T::Storage = TableStorage
    table_components: Option<ThinSlicePtr<'w, UnsafeCell<T>>>,
    table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>>,
    entity_table_rows: Option<ThinSlicePtr<'w, usize>>,
    // T::Storage = SparseStorage
    entities: Option<ThinSlicePtr<'w, Entity>>,
    sparse_set: Option<&'w ComponentSparseSet>,

    last_change_tick: u32,
    change_tick: u32,
}

impl<T> Clone for WriteFetch<'_, T> {
    fn clone(&self) -> Self {
        Self {
            table_components: self.table_components,
            table_ticks: self.table_ticks,
            entities: self.entities,
            entity_table_rows: self.entity_table_rows,
            sparse_set: self.sparse_set,
            last_change_tick: self.last_change_tick,
            change_tick: self.change_tick,
        }
    }
}

impl<'w, T: Component> WorldQueryGats<'w> for &mut T {
    type Fetch = WriteFetch<'w, T>;
    type _State = ComponentIdState<T>;
}

/// SAFETY: component access and archetype component access are properly updated to reflect that `T` is
/// read and write
unsafe impl<'w, T: Component> Fetch<'w> for WriteFetch<'w, T> {
    type Item = Mut<'w, T>;
    type State = ComponentIdState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init(
        world: &'w World,
        state: &ComponentIdState<T>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            table_components: None,
            entities: None,
            entity_table_rows: None,
            sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet).then(|| {
                world
                    .storages()
                    .sparse_sets
                    .get(state.component_id)
                    .unwrap()
            }),
            table_ticks: None,
            last_change_tick,
            change_tick,
        }
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &'w Archetype,
        tables: &'w Tables,
    ) {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                self.entity_table_rows = Some(archetype.entity_table_rows().into());
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = Some(column.get_data_slice().into());
                self.table_ticks = Some(column.get_ticks_slice().into());
            }
            StorageType::SparseSet => self.entities = Some(archetype.entities().into()),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        let column = table.get_column(state.component_id).unwrap();
        self.table_components = Some(column.get_data_slice().into());
        self.table_ticks = Some(column.get_ticks_slice().into());
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let (entity_table_rows, (table_components, table_ticks)) = self
                    .entity_table_rows
                    .zip(self.table_components.zip(self.table_ticks))
                    .unwrap_or_else(|| debug_checked_unreachable());
                let table_row = *entity_table_rows.get(archetype_index);
                Mut {
                    value: table_components.get(table_row).deref_mut(),
                    ticks: Ticks {
                        component_ticks: table_ticks.get(table_row).deref_mut(),
                        change_tick: self.change_tick,
                        last_change_tick: self.last_change_tick,
                    },
                }
            }
            StorageType::SparseSet => {
                let (entities, sparse_set) = self
                    .entities
                    .zip(self.sparse_set)
                    .unwrap_or_else(|| debug_checked_unreachable());
                let entity = *entities.get(archetype_index);
                let (component, component_ticks) = sparse_set
                    .get_with_ticks(entity)
                    .unwrap_or_else(|| debug_checked_unreachable());
                Mut {
                    value: component.assert_unique().deref_mut(),
                    ticks: Ticks {
                        component_ticks: component_ticks.deref_mut(),
                        change_tick: self.change_tick,
                        last_change_tick: self.last_change_tick,
                    },
                }
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let (table_components, table_ticks) = self
            .table_components
            .zip(self.table_ticks)
            .unwrap_or_else(|| debug_checked_unreachable());
        Mut {
            value: table_components.get(table_row).deref_mut(),
            ticks: Ticks {
                component_ticks: table_ticks.get(table_row).deref_mut(),
                change_tick: self.change_tick,
                last_change_tick: self.last_change_tick,
            },
        }
    }

    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_read(state.component_id),
            "&mut {} conflicts with a previous access in this query. Mutable component access must be unique.",
                std::any::type_name::<T>(),
        );
        access.add_write(state.component_id);
    }

    fn update_archetype_component_access(
        state: &Self::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(state.component_id)
        {
            access.add_write(archetype_component_id);
        }
    }
}

// SAFETY: defers to soundness of `T: WorldQuery` impl
unsafe impl<T: WorldQuery> WorldQuery for Option<T> {
    type ReadOnly = Option<T::ReadOnly>;
    type State = OptionState<T::State>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item.map(T::shrink)
    }
}

/// The [`Fetch`] of `Option<T>`.
#[doc(hidden)]
#[derive(Clone)]
pub struct OptionFetch<T> {
    fetch: T,
    matches: bool,
}

/// SAFETY: [`OptionFetch`] is read only because `T` is read only
unsafe impl<T: ReadOnlyWorldQuery> ReadOnlyWorldQuery for Option<T> {}

/// The [`FetchState`] of `Option<T>`.
#[doc(hidden)]
pub struct OptionState<T: FetchState> {
    state: T,
}

impl<T: FetchState> FetchState for OptionState<T> {
    fn init(world: &mut World) -> Self {
        Self {
            state: T::init(world),
        }
    }

    fn matches_component_set(&self, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        true
    }
}

impl<'w, T: WorldQueryGats<'w>> WorldQueryGats<'w> for Option<T> {
    type Fetch = OptionFetch<T::Fetch>;
    type _State = OptionState<T::_State>;
}

// SAFETY: component access and archetype component access are properly updated according to the
// internal Fetch
unsafe impl<'w, T: Fetch<'w>> Fetch<'w> for OptionFetch<T> {
    type Item = Option<T::Item>;
    type State = OptionState<T::State>;

    const IS_DENSE: bool = T::IS_DENSE;

    const IS_ARCHETYPAL: bool = T::IS_ARCHETYPAL;

    unsafe fn init(
        world: &'w World,
        state: &OptionState<T::State>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            fetch: T::init(world, &state.state, last_change_tick, change_tick),
            matches: false,
        }
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &'w Archetype,
        tables: &'w Tables,
    ) {
        self.matches = state
            .state
            .matches_component_set(&|id| archetype.contains(id));
        if self.matches {
            self.fetch.set_archetype(&state.state, archetype, tables);
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        self.matches = state
            .state
            .matches_component_set(&|id| table.has_column(id));
        if self.matches {
            self.fetch.set_table(&state.state, table);
        }
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        if self.matches {
            Some(self.fetch.archetype_fetch(archetype_index))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        if self.matches {
            Some(self.fetch.table_fetch(table_row))
        } else {
            None
        }
    }

    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        // We don't want to add the `with`/`without` of `T` as `Option<T>` will match things regardless of
        // `T`'s filters. for example `Query<(Option<&U>, &mut V)>` will match every entity with a `V` component
        // regardless of whether it has a `U` component. If we dont do this the query will not conflict with
        // `Query<&mut V, Without<U>>` which would be unsound.
        let mut intermediate = access.clone();
        T::update_component_access(&state.state, &mut intermediate);
        access.extend_access(&intermediate);
    }

    fn update_archetype_component_access(
        state: &Self::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if state.matches_component_set(&|id| archetype.contains(id)) {
            T::update_archetype_component_access(&state.state, archetype, access);
        }
    }
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
#[derive(Clone)]
pub struct ChangeTrackers<T: Component> {
    pub(crate) component_ticks: ComponentTicks,
    pub(crate) last_change_tick: u32,
    pub(crate) change_tick: u32,
    marker: PhantomData<T>,
}

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

// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for ChangeTrackers<T> {
    type ReadOnly = Self;
    type State = ChangeTrackersState<T>;

    fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
        item
    }
}

/// The [`FetchState`] of [`ChangeTrackers`].
#[doc(hidden)]
pub struct ChangeTrackersState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

impl<T: Component> FetchState for ChangeTrackersState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        set_contains_id(self.component_id)
    }
}

/// The [`Fetch`] of [`ChangeTrackers`].
#[doc(hidden)]
pub struct ChangeTrackersFetch<'w, T> {
    // T::Storage = TableStorage
    table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>>,
    entity_table_rows: Option<ThinSlicePtr<'w, usize>>,
    // T::Storage = SparseStorage
    entities: Option<ThinSlicePtr<'w, Entity>>,
    sparse_set: Option<&'w ComponentSparseSet>,

    marker: PhantomData<T>,
    last_change_tick: u32,
    change_tick: u32,
}

impl<T> Clone for ChangeTrackersFetch<'_, T> {
    fn clone(&self) -> Self {
        Self {
            table_ticks: self.table_ticks,
            entity_table_rows: self.entity_table_rows,
            entities: self.entities,
            sparse_set: self.sparse_set,
            marker: self.marker,
            last_change_tick: self.last_change_tick,
            change_tick: self.change_tick,
        }
    }
}

/// SAFETY: access is read only
unsafe impl<T: Component> ReadOnlyWorldQuery for ChangeTrackers<T> {}

impl<'w, T: Component> WorldQueryGats<'w> for ChangeTrackers<T> {
    type Fetch = ChangeTrackersFetch<'w, T>;
    type _State = ChangeTrackersState<T>;
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// read
unsafe impl<'w, T: Component> Fetch<'w> for ChangeTrackersFetch<'w, T> {
    type Item = ChangeTrackers<T>;
    type State = ChangeTrackersState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init(
        world: &'w World,
        state: &ChangeTrackersState<T>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> ChangeTrackersFetch<'w, T> {
        ChangeTrackersFetch {
            table_ticks: None,
            entities: None,
            entity_table_rows: None,
            sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet).then(|| {
                world
                    .storages()
                    .sparse_sets
                    .get(state.component_id)
                    .unwrap()
            }),
            marker: PhantomData,
            last_change_tick,
            change_tick,
        }
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &'w Archetype,
        tables: &'w Tables,
    ) {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                self.entity_table_rows = Some(archetype.entity_table_rows().into());
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_ticks = Some(column.get_ticks_slice().into());
            }
            StorageType::SparseSet => self.entities = Some(archetype.entities().into()),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        self.table_ticks = Some(
            table
                .get_column(state.component_id)
                .unwrap()
                .get_ticks_slice()
                .into(),
        );
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let entity_table_rows = self
                    .entity_table_rows
                    .unwrap_or_else(|| debug_checked_unreachable());
                let table_row = *entity_table_rows.get(archetype_index);
                ChangeTrackers {
                    component_ticks: {
                        let table_ticks = self
                            .table_ticks
                            .unwrap_or_else(|| debug_checked_unreachable());
                        table_ticks.get(table_row).read()
                    },
                    marker: PhantomData,
                    last_change_tick: self.last_change_tick,
                    change_tick: self.change_tick,
                }
            }
            StorageType::SparseSet => {
                let entities = self.entities.unwrap_or_else(|| debug_checked_unreachable());
                let entity = *entities.get(archetype_index);
                ChangeTrackers {
                    component_ticks: self
                        .sparse_set
                        .unwrap_or_else(|| debug_checked_unreachable())
                        .get_ticks(entity)
                        .map(|ticks| &*ticks.get())
                        .cloned()
                        .unwrap_or_else(|| debug_checked_unreachable()),
                    marker: PhantomData,
                    last_change_tick: self.last_change_tick,
                    change_tick: self.change_tick,
                }
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        ChangeTrackers {
            component_ticks: {
                let table_ticks = self
                    .table_ticks
                    .unwrap_or_else(|| debug_checked_unreachable());
                table_ticks.get(table_row).read()
            },
            marker: PhantomData,
            last_change_tick: self.last_change_tick,
            change_tick: self.change_tick,
        }
    }

    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_write(state.component_id),
            "ChangeTrackers<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>()
        );
        access.add_read(state.component_id);
    }

    fn update_archetype_component_access(
        state: &Self::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(state.component_id)
        {
            access.add_read(archetype_component_id);
        }
    }
}

macro_rules! impl_tuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, $($name: WorldQueryGats<'w>),*> WorldQueryGats<'w> for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
            type _State = ($($name::_State,)*);
        }

        #[allow(non_snake_case)]
        // SAFETY: update_component_access and update_archetype_component_access are called for each item in the tuple
        unsafe impl<'w, $($name: Fetch<'w>),*> Fetch<'w> for ($($name,)*) {
            type Item = ($($name::Item,)*);
            type State = ($($name::State,)*);

            #[allow(clippy::unused_unit)]
            unsafe fn init(_world: &'w World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                let ($($name,)*) = state;
                ($($name::init(_world, $name, _last_change_tick, _change_tick),)*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            const IS_ARCHETYPAL: bool = true $(&& $name::IS_ARCHETYPAL)*;

            #[inline]
            unsafe fn set_archetype(&mut self, _state: &Self::State, _archetype: &'w Archetype, _tables: &'w Tables) {
                let ($($name,)*) = self;
                let ($($state,)*) = _state;
                $($name.set_archetype($state, _archetype, _tables);)*
            }

            #[inline]
            unsafe fn set_table(&mut self, _state: &Self::State, _table: &'w Table) {
                let ($($name,)*) = self;
                let ($($state,)*) = _state;
                $($name.set_table($state, _table);)*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {
                let ($($name,)*) = self;
                ($($name.table_fetch(_table_row),)*)
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                let ($($name,)*) = self;
                ($($name.archetype_fetch(_archetype_index),)*)
            }

            #[allow(unused_variables)]
            #[inline]
            unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.table_filter_fetch(table_row))*
            }

            #[allow(unused_variables)]
            #[inline]
            unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.archetype_filter_fetch(archetype_index))*
            }

            fn update_component_access(state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = state;
                $($name::update_component_access($name, _access);)*
            }

            fn update_archetype_component_access(state: &Self::State, _archetype: &Archetype, _access: &mut Access<ArchetypeComponentId>) {
                let ($($name,)*) = state;
                $($name::update_archetype_component_access($name, _archetype, _access);)*
            }
        }

        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        impl<$($name: FetchState),*> FetchState for ($($name,)*) {
            fn init(_world: &mut World) -> Self {
                ($($name::init(_world),)*)
            }

            fn matches_component_set(&self, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.matches_component_set(_set_contains_id))*
            }
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
        }

        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyWorldQuery),*> ReadOnlyWorldQuery for ($($name,)*) {}

    };
}

/// The `AnyOf` query parameter fetches entities with any of the component types included in T.
///
/// `Query<AnyOf<(&A, &B, &mut C)>>` is equivalent to `Query<(Option<&A>, Option<&B>, Option<&mut C>), (Or(With<A>, With<B>, With<C>)>`.
/// Each of the components in `T` is returned as an `Option`, as with `Option<A>` queries.
/// Entities are guaranteed to have at least one of the components in `T`.
#[derive(Clone)]
pub struct AnyOf<T>(T);

macro_rules! impl_anytuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, $($name: WorldQueryGats<'w>),*> WorldQueryGats<'w> for AnyOf<($($name,)*)> {
            type Fetch = AnyOf<($(($name::Fetch, bool),)*)>;
            type _State = AnyOf<($($name::_State,)*)>;
        }

        #[allow(non_snake_case)]
        // SAFETY: update_component_access and update_archetype_component_access are called for each item in the tuple
        unsafe impl<'w, $($name: Fetch<'w>),*> Fetch<'w> for AnyOf<($(($name, bool),)*)> {
            type Item = ($(Option<$name::Item>,)*);
            type State = AnyOf<($($name::State,)*)>;

            #[allow(clippy::unused_unit)]
            unsafe fn init(_world: &'w World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                let ($($name,)*) = &state.0;
                AnyOf(($(($name::init(_world, $name, _last_change_tick, _change_tick), false),)*))
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            const IS_ARCHETYPAL: bool = true $(&& $name::IS_ARCHETYPAL)*;

            #[inline]
            unsafe fn set_archetype(&mut self, _state: &Self::State, _archetype: &'w Archetype, _tables: &'w Tables) {
                let ($($name,)*) = &mut self.0;
                let ($($state,)*) = &_state.0;
                $(
                    $name.1 = $state.matches_component_set(&|id| _archetype.contains(id));
                    if $name.1 {
                        $name.0.set_archetype($state, _archetype, _tables);
                    }
                )*
            }

            #[inline]
            unsafe fn set_table(&mut self, _state: &Self::State, _table: &'w Table) {
                let ($($name,)*) = &mut self.0;
                let ($($state,)*) = &_state.0;
                $(
                    $name.1 = $state.matches_component_set(&|id| _table.has_column(id));
                    if $name.1 {
                        $name.0.set_table($state, _table);
                    }
                )*
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {
                let ($($name,)*) = &mut self.0;
                ($(
                    $name.1.then(|| $name.0.table_fetch(_table_row)),
                )*)
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                let ($($name,)*) = &mut self.0;
                ($(
                    $name.1.then(|| $name.0.archetype_fetch(_archetype_index)),
                )*)
            }

            fn update_component_access(state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = &state.0;

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
                let ($($name,)*) = &state.0;
                $(
                    if $name.matches_component_set(&|id| _archetype.contains(id)) {
                        $name::update_archetype_component_access($name, _archetype, _access);
                    }
                )*
            }
        }

        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        impl<$($name: FetchState),*> FetchState for AnyOf<($($name,)*)> {
            fn init(_world: &mut World) -> Self {
                AnyOf(($($name::init(_world),)*))
            }

            fn matches_component_set(&self, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($name,)*) = &self.0;
                false $(|| $name.matches_component_set(_set_contains_id))*
            }
        }

        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        // SAFETY: defers to soundness of `$name: WorldQuery` impl
        unsafe impl<$($name: WorldQuery),*> WorldQuery for AnyOf<($($name,)*)> {
            type ReadOnly = AnyOf<($($name::ReadOnly,)*)>;
            type State = AnyOf<($($name::State,)*)>;

            fn shrink<'wlong: 'wshort, 'wshort>(item: QueryItem<'wlong, Self>) -> QueryItem<'wshort, Self> {
                let ($($name,)*) = item;
                ($(
                    $name.map($name::shrink),
                )*)
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
}
impl<'a, Q: WorldQuery> WorldQueryGats<'a> for NopWorldQuery<Q> {
    type Fetch = NopFetch<QueryFetch<'a, Q>>;
    type _State = <Q as WorldQueryGats<'a>>::_State;
}
/// SAFETY: `NopFetch` never accesses any data
unsafe impl<Q: WorldQuery> ReadOnlyWorldQuery for NopWorldQuery<Q> {}

/// [`Fetch`] that does not actually fetch anything
///
/// Mostly useful when something is generic over the Fetch and you don't want to fetch as you will discard the result
pub struct NopFetch<State> {
    state: PhantomData<State>,
}

// SAFETY: NopFetch doesnt access anything
unsafe impl<'w, F: Fetch<'w>> Fetch<'w> for NopFetch<F> {
    type Item = ();
    type State = F::State;

    const IS_DENSE: bool = F::IS_DENSE;

    const IS_ARCHETYPAL: bool = true;

    #[inline(always)]
    unsafe fn init(
        _world: &'w World,
        _state: &F::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        Self { state: PhantomData }
    }

    #[inline(always)]
    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        _archetype: &Archetype,
        _tables: &Tables,
    ) {
    }

    #[inline(always)]
    unsafe fn set_table(&mut self, _state: &Self::State, _table: &Table) {}

    #[inline(always)]
    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {}

    #[inline(always)]
    unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {}

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn update_archetype_component_access(
        _state: &Self::State,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }
}
