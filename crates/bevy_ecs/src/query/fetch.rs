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
use std::{cell::UnsafeCell, marker::PhantomData, mem::ManuallyDrop};

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
///     for (i1, i2) in my_query.iter_mut() {
///         let _: FooReadOnlyItem<'_> = i1;
///         let _: FooReadOnlyItem<'_> = i2;
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
/// struct EmptyQuery {
///     empty: (),
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
///     for _ in query.iter() {}
/// }
///
/// # bevy_ecs::system::assert_is_system(my_system);
/// ```
/// # Safety
///
/// component access of `ROQueryFetch<Self>` should be a subset of `QueryFetch<Self>`
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
pub unsafe trait ReadOnlyWorldQuery: WorldQuery {}

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
/// [`FetchState::matches_component_set`], [`Fetch::fetch`].
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
    /// for "dense" queries. If this returns true, [`Fetch::set_table`] before [`Fetch::fetch`]
    /// will be called for iterators. If this returns false, [`Fetch::set_archetype`] will be used
    /// before [`Fetch::fetch`] will be called for iterators.
    const IS_DENSE: bool;

    /// Returns true if (and only if) this Fetch relies strictly on archetypes to limit which
    /// components are acessed by the Query.
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

    /// Fetch [`Self::Item`] for either the given `entity` in the current [`Table`], or for the given
    /// `entity` in the current [`Archetype`]. This must always be called after [`Fetch::set_table`]
    /// with a `table_row` in the range of the current [`Table`] or after [`Fetch::set_archetype`]  
    /// with a `entity` in the current archetype.
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`Fetch::set_table`] or [`Fetch::set_archetype`]. `entity` and
    /// `table_row` must be in the range of the current table and archetype.
    unsafe fn fetch(&mut self, entity: Entity, table_index: usize) -> Self::Item;

    /// # Safety
    ///
    /// Must always be called _after_ [`Fetch::set_table`] or [`Fetch::set_archetype`]. `entity` and
    /// `table_row` must be in the range of the current table and archetype.
    #[allow(unused_variables)]
    #[inline(always)]
    unsafe fn filter_fetch(&mut self, entity: Entity, table_index: usize) -> bool {
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
pub struct EntityFetch;

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
    type Fetch = EntityFetch;
    type _State = EntityState;
}

/// SAFETY: no component or archetype access
unsafe impl<'w> Fetch<'w> for EntityFetch {
    type Item = Entity;
    type State = EntityState;

    const IS_DENSE: bool = true;

    const IS_ARCHETYPAL: bool = true;

    unsafe fn init(
        _world: &'w World,
        _state: &EntityState,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> EntityFetch {
        EntityFetch
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        _archetype: &'w Archetype,
        _tables: &Tables,
    ) {
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, _table: &'w Table) {}

    #[inline(always)]
    unsafe fn fetch(&mut self, entity: Entity, _table_row: usize) -> Self::Item {
        entity
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
    components: StorageSwitch<
        T,
        // T::Storage = TableStorage
        Option<ThinSlicePtr<'w, UnsafeCell<T>>>,
        // T::Storage = SparseStorage
        &'w ComponentSparseSet,
    >,
}

impl<T: Component> Clone for ReadFetch<'_, T> {
    fn clone(&self) -> Self {
        Self {
            components: self.components.clone(),
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
            components: if Self::IS_DENSE {
                StorageSwitch::new_table(None)
            } else {
                StorageSwitch::new_sparse_set(
                    world
                        .storages()
                        .sparse_sets
                        .get(state.component_id)
                        .unwrap_or_else(|| debug_checked_unreachable()),
                )
            },
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
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.components = StorageSwitch::new_table(Some(column.get_data_slice().into()));
            }
            StorageType::SparseSet => {}
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        if Self::IS_DENSE {
            self.components = StorageSwitch::new_table(Some(
                table
                    .get_column(state.component_id)
                    .unwrap_or_else(|| debug_checked_unreachable())
                    .get_data_slice()
                    .into(),
            ));
        }
    }

    #[inline(always)]
    unsafe fn fetch(&mut self, entity: Entity, table_row: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => self
                .components
                .table()
                .unwrap_or_else(|| debug_checked_unreachable())
                .get(table_row)
                .deref(),
            StorageType::SparseSet => self
                .components
                .sparse_set()
                .get(entity)
                .unwrap_or_else(|| debug_checked_unreachable())
                .deref(),
        }
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
    components: StorageSwitch<
        T,
        // T::Storage = TableStorage
        Option<(
            ThinSlicePtr<'w, UnsafeCell<T>>,
            ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>,
        )>,
        // T::Storage = SparseStorage
        &'w ComponentSparseSet,
    >,

    last_change_tick: u32,
    change_tick: u32,
}

impl<T: Component> Clone for WriteFetch<'_, T> {
    fn clone(&self) -> Self {
        Self {
            components: self.components.clone(),
            last_change_tick: self.last_change_tick,
            change_tick: self.change_tick,
        }
    }
}

impl<'w, T: Component> WorldQueryGats<'w> for &mut T {
    type Fetch = WriteFetch<'w, T>;
    type _State = ComponentIdState<T>;
}

/// SAFETY: component access and archetype component access are properly updated to reflect that T is
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
            components: if Self::IS_DENSE {
                StorageSwitch::new_table(None)
            } else {
                StorageSwitch::new_sparse_set(
                    world
                        .storages()
                        .sparse_sets
                        .get(state.component_id)
                        .unwrap_or_else(|| debug_checked_unreachable()),
                )
            },
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
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.components = StorageSwitch::new_table(Some((
                    column.get_data_slice().into(),
                    column.get_ticks_slice().into(),
                )));
            }
            StorageType::SparseSet => {}
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        if Self::IS_DENSE {
            let column = table.get_column(state.component_id).unwrap();
            self.components = StorageSwitch::new_table(Some((
                column.get_data_slice().into(),
                column.get_ticks_slice().into(),
            )));
        }
    }

    #[inline(always)]
    unsafe fn fetch(&mut self, entity: Entity, table_row: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let (table_components, table_ticks) = self
                    .components
                    .table()
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
            StorageType::SparseSet => {
                let (component, component_ticks) = self
                    .components
                    .sparse_set()
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

    #[inline(always)]
    unsafe fn fetch(&mut self, entity: Entity, table_row: usize) -> Self::Item {
        self.matches.then(|| self.fetch.fetch(entity, table_row))
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
    ticks: StorageSwitch<
        T,
        // T::Storage = TableStorage
        Option<ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>>,
        // T::Storage = SparseStorage
        &'w ComponentSparseSet,
    >,
    last_change_tick: u32,
    change_tick: u32,
}

impl<T: Component> Clone for ChangeTrackersFetch<'_, T> {
    fn clone(&self) -> Self {
        Self {
            ticks: self.ticks.clone(),
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
            ticks: if Self::IS_DENSE {
                StorageSwitch::new_table(None)
            } else {
                StorageSwitch::new_sparse_set(
                    world
                        .storages()
                        .sparse_sets
                        .get(state.component_id)
                        .unwrap_or_else(|| debug_checked_unreachable()),
                )
            },
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
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.ticks = StorageSwitch::new_table(Some(column.get_ticks_slice().into()));
            }
            StorageType::SparseSet => {}
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        if Self::IS_DENSE {
            self.ticks = StorageSwitch::new_table(Some(
                table
                    .get_column(state.component_id)
                    .unwrap()
                    .get_ticks_slice()
                    .into(),
            ));
        }
    }

    #[inline(always)]
    unsafe fn fetch(&mut self, entity: Entity, table_row: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => ChangeTrackers {
                component_ticks: {
                    let table_ticks = self
                        .ticks
                        .table()
                        .unwrap_or_else(|| debug_checked_unreachable());
                    table_ticks.get(table_row).read()
                },
                marker: PhantomData,
                last_change_tick: self.last_change_tick,
                change_tick: self.change_tick,
            },
            StorageType::SparseSet => ChangeTrackers {
                component_ticks: self
                    .ticks
                    .sparse_set()
                    .get_ticks(entity)
                    .map(|ticks| &*ticks.get())
                    .cloned()
                    .unwrap_or_else(|| debug_checked_unreachable()),
                marker: PhantomData,
                last_change_tick: self.last_change_tick,
                change_tick: self.change_tick,
            },
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

            #[inline(always)]
            #[allow(clippy::unused_unit)]
            unsafe fn fetch(&mut self, _entity: Entity, _table_row: usize) -> Self::Item {
                let ($($name,)*) = self;
                ($($name.fetch(_entity, _table_row),)*)
            }

            #[inline(always)]
            unsafe fn filter_fetch(&mut self, _entity: Entity, _table_row: usize) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.filter_fetch(_entity, _table_row))*
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

            #[inline(always)]
            #[allow(clippy::unused_unit)]
            unsafe fn fetch(&mut self, _entity: Entity, _table_row: usize) -> Self::Item {
                let ($($name,)*) = &mut self.0;
                ($(
                    $name.1.then(|| $name.0.fetch(_entity, _table_row)),
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

/// [`Fetch`] that does not actually fetch anything
///
/// Mostly useful when something is generic over the Fetch and you don't want to fetch as you will discard the result
pub struct NopFetch<State> {
    state: PhantomData<State>,
}

// SAFETY: NopFetch doesnt access anything
unsafe impl<'w, State: FetchState> Fetch<'w> for NopFetch<State> {
    type Item = ();
    type State = State;

    const IS_DENSE: bool = true;

    const IS_ARCHETYPAL: bool = true;

    #[inline(always)]
    unsafe fn init(
        _world: &'w World,
        _state: &State,
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
    unsafe fn fetch(&mut self, _entity: Entity, _table_row: usize) -> Self::Item {}

    fn update_component_access(_state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {}

    fn update_archetype_component_access(
        _state: &Self::State,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }
}

pub(super) union StorageSwitch<T, A, B> {
    table: ManuallyDrop<A>,
    sparse_set: ManuallyDrop<B>,
    marker: PhantomData<T>,
}

impl<T: Component, A, B> StorageSwitch<T, A, B> {
    pub const fn new_table(table: A) -> Self {
        Self {
            table: ManuallyDrop::new(table),
        }
    }

    pub const fn new_sparse_set(sparse_set: B) -> Self {
        Self {
            sparse_set: ManuallyDrop::new(sparse_set),
        }
    }
}

impl<T: Component, A: Copy, B: Copy> StorageSwitch<T, A, B> {
    pub fn table(&self) -> A {
        unsafe {
            match T::Storage::STORAGE_TYPE {
                StorageType::Table => *self.table,
                _ => debug_checked_unreachable(),
            }
        }
    }

    pub fn sparse_set(&self) -> B {
        unsafe {
            match T::Storage::STORAGE_TYPE {
                StorageType::SparseSet => *self.sparse_set,
                _ => debug_checked_unreachable(),
            }
        }
    }
}

impl<T: Component, A: Clone, B: Clone> Clone for StorageSwitch<T, A, B> {
    fn clone(&self) -> Self {
        unsafe {
            match T::Storage::STORAGE_TYPE {
                StorageType::Table => Self {
                    table: self.table.clone(),
                },
                StorageType::SparseSet => Self {
                    sparse_set: self.sparse_set.clone(),
                },
            }
        }
    }
}

impl<T: Component, A: Copy, B: Copy> Copy for StorageSwitch<T, A, B> {}
