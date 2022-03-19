use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    change_detection::Ticks,
    component::{Component, ComponentId, ComponentStorage, ComponentTicks, StorageType},
    entity::Entity,
    query::{Access, FilteredAccess},
    storage::{ComponentSparseSet, Table, Tables},
    world::{Mut, World},
};
use bevy_ecs_macros::all_tuples;
pub use bevy_ecs_macros::WorldQuery;
use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    ptr::{self, NonNull},
};

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
/// structs that implement [`Fetch`] and [`FetchState`] and are used as [`WorldQuery::Fetch`] and
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
/// struct MyQuery<'w> {
///     entity: Entity,
///     foo: &'w Foo,
///     bar: Option<&'w Bar>,
/// }
///
/// fn my_system(query: Query<MyQuery>) {
///     for q in query.iter() {
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
/// struct HealthQuery<'w> {
///     health: &'w mut Health,
///     buff: Option<&'w mut Buff>,
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
///     for health in health_query.iter() {
///         println!("Total: {}", health.total());
///     }
///     // Iterator's item is `HealthQueryItem`.
///     for mut health in health_query.iter_mut() {
///         health.damage(1.0);
///         println!("Total (mut): {}", health.total());
///     }
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// **Note:** if you omit the `mutable` attribute for a query that doesn't implement
/// `ReadOnlyFetch`, compilation will fail. We insert static checks as in the example above for
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
/// struct FooQuery<'w> {
///     foo: &'w Foo,
///     bar_query: BarQuery<'w>,
/// }
///
/// #[derive(WorldQuery)]
/// #[world_query(mutable)]
/// struct BarQuery<'w> {
///     bar: &'w mut Bar,
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
/// struct FooQuery<'w> {
///     foo: &'w Foo,
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
/// struct MyQuery<'w> {
///     foo: FooQuery<'w>,
///     bar: (&'w Bar, Option<&'w OptionalBar>)
/// }
///
/// #[derive(WorldQuery)]
/// struct FooQuery<'w> {
///     foo: &'w Foo,
///     optional_foo: Option<&'w OptionalFoo>,
/// }
///
/// // You can also compose derived queries with regular ones in tuples.
/// fn my_system(query: Query<(&Foo, MyQuery, FooQuery)>) {
///     for (foo, my_query, foo_query) in query.iter() {
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
/// struct EmptyQuery<'w> {
///     #[world_query(ignore)]
///     _w: std::marker::PhantomData<&'w ()>,
/// }
///
/// fn my_system(query: Query<EmptyQuery>) {
///     for _ in query.iter() {}
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
///
/// ## Filters
///
/// Using [`derive@super::WorldQuery`] macro in conjunctions with the `#[world_query(filter)]`
/// attribute allows creating custom query filters.
///
/// To do so, all fields in the struct must be filters themselves (their [`WorldQuery::Fetch`]
/// associated types should implement [`super::FilterFetch`]).
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
/// #[world_query(filter)]
/// struct MyFilter<T: Component, P: Component> {
///     _foo: With<Foo>,
///     _bar: With<Bar>,
///     _or: Or<(With<Baz>, Changed<Foo>, Added<Bar>)>,
///     _generic_tuple: (With<T>, Without<P>),
///     #[world_query(ignore)]
///     _tp: std::marker::PhantomData<(T, P)>,
/// }
///
/// fn my_system(query: Query<Entity, MyFilter<Foo, Qux>>) {
///     for _ in query.iter() {}
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
pub trait WorldQuery {
    type Fetch: for<'world, 'state> Fetch<'world, 'state, State = Self::State>;
    type State: FetchState;
    type ReadOnlyFetch: for<'world, 'state> Fetch<'world, 'state, State = Self::State>
        + ReadOnlyFetch;
}

pub type QueryItem<'w, 's, Q> = <<Q as WorldQuery>::Fetch as Fetch<'w, 's>>::Item;

/// Types that implement this trait are responsible for fetching query items from tables or
/// archetypes.
///
/// Every type that implements [`WorldQuery`] have their associated [`WorldQuery::Fetch`] and
/// [`WorldQuery::State`] types that are essential for fetching component data.
pub trait Fetch<'world, 'state>: Sized {
    type Item;
    type State: FetchState;

    /// Creates a new instance of this fetch.
    ///
    /// # Safety
    ///
    /// `state` must have been initialized (via [`FetchState::init`]) using the same `world` passed
    /// in to this function.
    unsafe fn init(
        world: &World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self;

    /// Returns true if (and only if) every table of every archetype matched by this Fetch contains
    /// all of the matched components. This is used to select a more efficient "table iterator"
    /// for "dense" queries. If this returns true, [`Fetch::set_table`] and [`Fetch::table_fetch`]
    /// will be called for iterators. If this returns false, [`Fetch::set_archetype`] and
    /// [`Fetch::archetype_fetch`] will be called for iterators.
    const IS_DENSE: bool;

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `archetype` and `tables` must be from the [`World`] [`Fetch::init`] was called on. `state`
    /// must be the [`Self::State`] this was initialized with.
    unsafe fn set_archetype(&mut self, state: &Self::State, archetype: &Archetype, tables: &Tables);

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `table` must be from the [`World`] [`Fetch::init`] was called on. `state` must be the
    /// [`Self::State`] this was initialized with.
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table);

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
}

/// State used to construct a Fetch. This will be cached inside [`QueryState`](crate::query::QueryState),
///  so it is best to move as much data / computation here as possible to reduce the cost of
/// constructing Fetch.
///
/// # Safety
///
/// Implementor must ensure that [`FetchState::update_component_access`] and
/// [`FetchState::update_archetype_component_access`] exactly reflects the results of
/// [`FetchState::matches_archetype`], [`FetchState::matches_table`], [`Fetch::archetype_fetch`], and
/// [`Fetch::table_fetch`].
pub unsafe trait FetchState: Send + Sync + Sized {
    fn init(world: &mut World) -> Self;
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>);
    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    );
    fn matches_archetype(&self, archetype: &Archetype) -> bool;
    fn matches_table(&self, table: &Table) -> bool;
}

/// A fetch that is read only.
///
/// # Safety
///
/// This must only be implemented for read-only fetches.
pub unsafe trait ReadOnlyFetch {}

impl WorldQuery for Entity {
    type Fetch = EntityFetch;
    type State = EntityState;
    type ReadOnlyFetch = EntityFetch;
}

/// The [`Fetch`] of [`Entity`].
#[doc(hidden)]
#[derive(Clone)]
pub struct EntityFetch {
    entities: *const Entity,
}

/// SAFETY: access is read only
unsafe impl ReadOnlyFetch for EntityFetch {}

/// The [`FetchState`] of [`Entity`].
#[doc(hidden)]
pub struct EntityState;

// SAFETY: no component or archetype access
unsafe impl FetchState for EntityState {
    fn init(_world: &mut World) -> Self {
        Self
    }

    fn update_component_access(&self, _access: &mut FilteredAccess<ComponentId>) {}

    fn update_archetype_component_access(
        &self,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    #[inline]
    fn matches_archetype(&self, _archetype: &Archetype) -> bool {
        true
    }

    #[inline]
    fn matches_table(&self, _table: &Table) -> bool {
        true
    }
}

impl<'w, 's> Fetch<'w, 's> for EntityFetch {
    type Item = Entity;
    type State = EntityState;

    const IS_DENSE: bool = true;

    unsafe fn init(
        _world: &World,
        _state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        Self {
            entities: std::ptr::null::<Entity>(),
        }
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        archetype: &Archetype,
        _tables: &Tables,
    ) {
        self.entities = archetype.entities().as_ptr();
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, table: &Table) {
        self.entities = table.entities().as_ptr();
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        *self.entities.add(table_row)
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        *self.entities.add(archetype_index)
    }
}

impl<T: Component> WorldQuery for &T {
    type Fetch = ReadFetch<T>;
    type State = ReadState<T>;
    type ReadOnlyFetch = ReadFetch<T>;
}

/// The [`FetchState`] of `&T`.
#[doc(hidden)]
pub struct ReadState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// read
unsafe impl<T: Component> FetchState for ReadState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        ReadState {
            component_id,
            marker: PhantomData,
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_write(self.component_id),
            "&{} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>(),
        );
        access.add_read(self.component_id);
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(self.component_id)
        {
            access.add_read(archetype_component_id);
        }
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.component_id)
    }

    fn matches_table(&self, table: &Table) -> bool {
        table.has_column(self.component_id)
    }
}

/// The [`Fetch`] of `&T`.
#[doc(hidden)]
pub struct ReadFetch<T> {
    table_components: NonNull<T>,
    entity_table_rows: *const usize,
    entities: *const Entity,
    sparse_set: *const ComponentSparseSet,
}

impl<T> Clone for ReadFetch<T> {
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
unsafe impl<T> ReadOnlyFetch for ReadFetch<T> {}

impl<'w, 's, T: Component> Fetch<'w, 's> for ReadFetch<T> {
    type Item = &'w T;
    type State = ReadState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &World,
        state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        let mut value = Self {
            table_components: NonNull::dangling(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
        };
        if T::Storage::STORAGE_TYPE == StorageType::SparseSet {
            value.sparse_set = world
                .storages()
                .sparse_sets
                .get(state.component_id)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = column.get_data_ptr().cast::<T>();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        self.table_components = table
            .get_column(state.component_id)
            .unwrap()
            .get_data_ptr()
            .cast::<T>();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let table_row = *self.entity_table_rows.add(archetype_index);
                &*self.table_components.as_ptr().add(table_row)
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                &*(*self.sparse_set).get(entity).unwrap().cast::<T>()
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        &*self.table_components.as_ptr().add(table_row)
    }
}

impl<T: Component> WorldQuery for &mut T {
    type Fetch = WriteFetch<T>;
    type State = WriteState<T>;
    type ReadOnlyFetch = ReadOnlyWriteFetch<T>;
}

/// The [`Fetch`] of `&mut T`.
#[doc(hidden)]
pub struct WriteFetch<T> {
    table_components: NonNull<T>,
    table_ticks: *const UnsafeCell<ComponentTicks>,
    entities: *const Entity,
    entity_table_rows: *const usize,
    sparse_set: *const ComponentSparseSet,
    last_change_tick: u32,
    change_tick: u32,
}

impl<T> Clone for WriteFetch<T> {
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

/// The [`ReadOnlyFetch`] of `&mut T`.
#[doc(hidden)]
pub struct ReadOnlyWriteFetch<T> {
    table_components: NonNull<T>,
    entities: *const Entity,
    entity_table_rows: *const usize,
    sparse_set: *const ComponentSparseSet,
}

/// SAFETY: access is read only
unsafe impl<T> ReadOnlyFetch for ReadOnlyWriteFetch<T> {}

impl<T> Clone for ReadOnlyWriteFetch<T> {
    fn clone(&self) -> Self {
        Self {
            table_components: self.table_components,
            entities: self.entities,
            entity_table_rows: self.entity_table_rows,
            sparse_set: self.sparse_set,
        }
    }
}

/// The [`FetchState`] of `&mut T`.
#[doc(hidden)]
pub struct WriteState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// written
unsafe impl<T: Component> FetchState for WriteState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        WriteState {
            component_id,
            marker: PhantomData,
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_read(self.component_id),
            "&mut {} conflicts with a previous access in this query. Mutable component access must be unique.",
                std::any::type_name::<T>(),
        );
        access.add_write(self.component_id);
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(self.component_id)
        {
            access.add_write(archetype_component_id);
        }
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.component_id)
    }

    fn matches_table(&self, table: &Table) -> bool {
        table.has_column(self.component_id)
    }
}

impl<'w, 's, T: Component> Fetch<'w, 's> for WriteFetch<T> {
    type Item = Mut<'w, T>;
    type State = WriteState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        let mut value = Self {
            table_components: NonNull::dangling(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
            table_ticks: ptr::null::<UnsafeCell<ComponentTicks>>(),
            last_change_tick,
            change_tick,
        };
        if T::Storage::STORAGE_TYPE == StorageType::SparseSet {
            value.sparse_set = world
                .storages()
                .sparse_sets
                .get(state.component_id)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = column.get_data_ptr().cast::<T>();
                self.table_ticks = column.get_ticks_ptr();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        let column = table.get_column(state.component_id).unwrap();
        self.table_components = column.get_data_ptr().cast::<T>();
        self.table_ticks = column.get_ticks_ptr();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let table_row = *self.entity_table_rows.add(archetype_index);
                Mut {
                    value: &mut *self.table_components.as_ptr().add(table_row),
                    ticks: Ticks {
                        component_ticks: &mut *(*self.table_ticks.add(table_row)).get(),
                        change_tick: self.change_tick,
                        last_change_tick: self.last_change_tick,
                    },
                }
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                let (component, component_ticks) =
                    (*self.sparse_set).get_with_ticks(entity).unwrap();
                Mut {
                    value: &mut *component.cast::<T>(),
                    ticks: Ticks {
                        component_ticks: &mut *component_ticks,
                        change_tick: self.change_tick,
                        last_change_tick: self.last_change_tick,
                    },
                }
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        Mut {
            value: &mut *self.table_components.as_ptr().add(table_row),
            ticks: Ticks {
                component_ticks: &mut *(*self.table_ticks.add(table_row)).get(),
                change_tick: self.change_tick,
                last_change_tick: self.last_change_tick,
            },
        }
    }
}

impl<'w, 's, T: Component> Fetch<'w, 's> for ReadOnlyWriteFetch<T> {
    type Item = &'w T;
    type State = WriteState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &World,
        state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        let mut value = Self {
            table_components: NonNull::dangling(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
        };
        if T::Storage::STORAGE_TYPE == StorageType::SparseSet {
            value.sparse_set = world
                .storages()
                .sparse_sets
                .get(state.component_id)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = column.get_data_ptr().cast::<T>();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        let column = table.get_column(state.component_id).unwrap();
        self.table_components = column.get_data_ptr().cast::<T>();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let table_row = *self.entity_table_rows.add(archetype_index);
                &*self.table_components.as_ptr().add(table_row)
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                &*(*self.sparse_set).get(entity).unwrap().cast::<T>()
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        &*self.table_components.as_ptr().add(table_row)
    }
}

impl<T: WorldQuery> WorldQuery for Option<T> {
    type Fetch = OptionFetch<T::Fetch>;
    type State = OptionState<T::State>;
    type ReadOnlyFetch = OptionFetch<T::ReadOnlyFetch>;
}

/// The [`Fetch`] of `Option<T>`.
#[doc(hidden)]
#[derive(Clone)]
pub struct OptionFetch<T> {
    fetch: T,
    matches: bool,
}

/// SAFETY: [`OptionFetch`] is read only because `T` is read only
unsafe impl<T: ReadOnlyFetch> ReadOnlyFetch for OptionFetch<T> {}

/// The [`FetchState`] of `Option<T>`.
#[doc(hidden)]
pub struct OptionState<T: FetchState> {
    state: T,
}

// SAFETY: component access and archetype component access are properly updated according to the
// internal Fetch
unsafe impl<T: FetchState> FetchState for OptionState<T> {
    fn init(world: &mut World) -> Self {
        Self {
            state: T::init(world),
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        self.state.update_component_access(access);
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if self.state.matches_archetype(archetype) {
            self.state
                .update_archetype_component_access(archetype, access);
        }
    }

    fn matches_archetype(&self, _archetype: &Archetype) -> bool {
        true
    }

    fn matches_table(&self, _table: &Table) -> bool {
        true
    }
}

impl<'w, 's, T: Fetch<'w, 's>> Fetch<'w, 's> for OptionFetch<T> {
    type Item = Option<T::Item>;
    type State = OptionState<T::State>;

    const IS_DENSE: bool = T::IS_DENSE;

    unsafe fn init(
        world: &World,
        state: &Self::State,
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
        archetype: &Archetype,
        tables: &Tables,
    ) {
        self.matches = state.state.matches_archetype(archetype);
        if self.matches {
            self.fetch.set_archetype(&state.state, archetype, tables);
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        self.matches = state.state.matches_table(table);
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
///     for (name, tracker) in query.iter() {
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

impl<T: Component> WorldQuery for ChangeTrackers<T> {
    type Fetch = ChangeTrackersFetch<T>;
    type State = ChangeTrackersState<T>;
    type ReadOnlyFetch = ChangeTrackersFetch<T>;
}

/// The [`FetchState`] of [`ChangeTrackers`].
#[doc(hidden)]
pub struct ChangeTrackersState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// read
unsafe impl<T: Component> FetchState for ChangeTrackersState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        assert!(
            !access.access().has_write(self.component_id),
            "ChangeTrackers<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>()
        );
        access.add_read(self.component_id);
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if let Some(archetype_component_id) =
            archetype.get_archetype_component_id(self.component_id)
        {
            access.add_read(archetype_component_id);
        }
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.component_id)
    }

    fn matches_table(&self, table: &Table) -> bool {
        table.has_column(self.component_id)
    }
}

/// The [`Fetch`] of [`ChangeTrackers`].
#[doc(hidden)]
pub struct ChangeTrackersFetch<T> {
    table_ticks: *const ComponentTicks,
    entity_table_rows: *const usize,
    entities: *const Entity,
    sparse_set: *const ComponentSparseSet,
    marker: PhantomData<T>,
    last_change_tick: u32,
    change_tick: u32,
}

impl<T> Clone for ChangeTrackersFetch<T> {
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
unsafe impl<T> ReadOnlyFetch for ChangeTrackersFetch<T> {}

impl<'w, 's, T: Component> Fetch<'w, 's> for ChangeTrackersFetch<T> {
    type Item = ChangeTrackers<T>;
    type State = ChangeTrackersState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &World,
        state: &Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        let mut value = Self {
            table_ticks: ptr::null::<ComponentTicks>(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
            marker: PhantomData,
            last_change_tick,
            change_tick,
        };
        if T::Storage::STORAGE_TYPE == StorageType::SparseSet {
            value.sparse_set = world
                .storages()
                .sparse_sets
                .get(state.component_id)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_ticks = column.get_ticks_const_ptr();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        self.table_ticks = table
            .get_column(state.component_id)
            .unwrap()
            .get_ticks_const_ptr();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let table_row = *self.entity_table_rows.add(archetype_index);
                ChangeTrackers {
                    component_ticks: (*self.table_ticks.add(table_row)).clone(),
                    marker: PhantomData,
                    last_change_tick: self.last_change_tick,
                    change_tick: self.change_tick,
                }
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                ChangeTrackers {
                    component_ticks: (*self.sparse_set).get_ticks(entity).cloned().unwrap(),
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
            component_ticks: (*self.table_ticks.add(table_row)).clone(),
            marker: PhantomData,
            last_change_tick: self.last_change_tick,
            change_tick: self.change_tick,
        }
    }
}

macro_rules! impl_tuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(non_snake_case)]
        impl<'w, 's, $($name: Fetch<'w, 's>),*> Fetch<'w, 's> for ($($name,)*) {
            type Item = ($($name::Item,)*);
            type State = ($($name::State,)*);

            #[allow(clippy::unused_unit)]
            unsafe fn init(_world: &World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                let ($($name,)*) = state;
                ($($name::init(_world, $name, _last_change_tick, _change_tick),)*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            #[inline]
            unsafe fn set_archetype(&mut self, _state: &Self::State, _archetype: &Archetype, _tables: &Tables) {
                let ($($name,)*) = self;
                let ($($state,)*) = _state;
                $($name.set_archetype($state, _archetype, _tables);)*
            }

            #[inline]
            unsafe fn set_table(&mut self, _state: &Self::State, _table: &Table) {
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
        }

        // SAFETY: update_component_access and update_archetype_component_access are called for each item in the tuple
        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        unsafe impl<$($name: FetchState),*> FetchState for ($($name,)*) {
            fn init(_world: &mut World) -> Self {
                ($($name::init(_world),)*)
            }

            fn update_component_access(&self, _access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = self;
                $($name.update_component_access(_access);)*
            }

            fn update_archetype_component_access(&self, _archetype: &Archetype, _access: &mut Access<ArchetypeComponentId>) {
                let ($($name,)*) = self;
                $($name.update_archetype_component_access(_archetype, _access);)*
            }

            fn matches_archetype(&self, _archetype: &Archetype) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.matches_archetype(_archetype))*
            }

            fn matches_table(&self, _table: &Table) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.matches_table(_table))*
            }
        }

        impl<$($name: WorldQuery),*> WorldQuery for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
            type State = ($($name::State,)*);
            type ReadOnlyFetch = ($($name::ReadOnlyFetch,)*);
        }

        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyFetch),*> ReadOnlyFetch for ($($name,)*) {}

    };
}

/// The `AnyOf` query parameter fetches entities with any of the component types included in T.
///
/// `Query<AnyOf<(&A, &B, &mut C)>>` is equivalent to `Query<(Option<&A>, Option<&B>, Option<&mut C>), (Or(With<A>, With<B>, With<C>)>`.
/// Each of the components in `T` is returned as an `Option`, as with `Option<A>` queries.
/// Entities are guaranteed to have at least one of the components in `T`.
pub struct AnyOf<T>(T);

macro_rules! impl_anytuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(non_snake_case)]
        impl<'w, 's, $($name: Fetch<'w, 's>),*> Fetch<'w, 's> for AnyOf<($(($name, bool),)*)> {
            type Item = ($(Option<$name::Item>,)*);
            type State = AnyOf<($($name::State,)*)>;

            #[allow(clippy::unused_unit)]
            unsafe fn init(_world: &World, state: &Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                let ($($name,)*) = &state.0;
                AnyOf(($(($name::init(_world, $name, _last_change_tick, _change_tick), false),)*))
            }


            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            #[inline]
            unsafe fn set_archetype(&mut self, _state: &Self::State, _archetype: &Archetype, _tables: &Tables) {
                let ($($name,)*) = &mut self.0;
                let ($($state,)*) = &_state.0;
                $(
                    $name.1 = $state.matches_archetype(_archetype);
                    if $name.1 {
                        $name.0.set_archetype($state, _archetype, _tables);
                    }
                )*
            }

            #[inline]
            unsafe fn set_table(&mut self, _state: &Self::State, _table: &Table) {
                let ($($name,)*) = &mut self.0;
                let ($($state,)*) = &_state.0;
                $(
                    $name.1 = $state.matches_table(_table);
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
        }

        // SAFETY: update_component_access and update_archetype_component_access are called for each item in the tuple
        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        unsafe impl<$($name: FetchState),*> FetchState for AnyOf<($($name,)*)> {
            fn init(_world: &mut World) -> Self {
                AnyOf(($($name::init(_world),)*))
            }

            fn update_component_access(&self, _access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = &self.0;
                $($name.update_component_access(_access);)*
            }

            fn update_archetype_component_access(&self, _archetype: &Archetype, _access: &mut Access<ArchetypeComponentId>) {
                let ($($name,)*) = &self.0;
                $(
                    if $name.matches_archetype(_archetype) {
                        $name.update_archetype_component_access(_archetype, _access);
                    }
                )*
            }

            fn matches_archetype(&self, _archetype: &Archetype) -> bool {
                let ($($name,)*) = &self.0;
                false $(|| $name.matches_archetype(_archetype))*
            }

            fn matches_table(&self, _table: &Table) -> bool {
                let ($($name,)*) = &self.0;
                false $(|| $name.matches_table(_table))*
            }
        }

        impl<$($name: WorldQuery),*> WorldQuery for AnyOf<($($name,)*)> {
            type Fetch = AnyOf<($(($name::Fetch, bool),)*)>;
            type ReadOnlyFetch = AnyOf<($(($name::ReadOnlyFetch, bool),)*)>;

            type State = AnyOf<($($name::State,)*)>;
        }

        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyFetch),*> ReadOnlyFetch for AnyOf<($(($name, bool),)*)> {}

    };
}

all_tuples!(impl_tuple_fetch, 0, 15, F, S);
all_tuples!(impl_anytuple_fetch, 0, 15, F, S);

/// [`Fetch`] that does not actually fetch anything
///
/// Mostly useful when something is generic over the Fetch and you don't want to fetch as you will discard the result
pub struct NopFetch<State> {
    state: PhantomData<State>,
}

impl<'w, 's, State: FetchState> Fetch<'w, 's> for NopFetch<State> {
    type Item = ();
    type State = State;

    const IS_DENSE: bool = true;

    #[inline(always)]
    unsafe fn init(
        _world: &World,
        _state: &Self::State,
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
}
