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
/// # Basic WorldQueries
///
/// Here is a small list of the most important world queries to know about where `C` stands for a
/// [`Component`] and `WQ` stands for a [`WorldQuery`]:
/// - `&C`: Queries immutably for the component `C`
/// - `&mut C`: Queries mutably for the component `C`
/// - `Option<WQ>`: Queries the inner WorldQuery `WQ` but instead of discarding the entity if the world
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
pub trait WorldQuery {
    type Fetch: for<'world, 'state> Fetch<'world, 'state, State = Self::State>;
    type State: FetchState;
}

pub trait Fetch<'world, 'state>: Sized {
    type Item;
    type State: FetchState;

    /// Creates a new instance of this fetch.
    ///
    /// # Safety
    ///
    /// `state` must have been initialized (via [FetchState::init]) using the same `world` passed in
    /// to this function.
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
    /// `archetype` and `tables` must be from the [`World`] [`Fetch::init`] was called on. `state` must
    /// be the [Self::State] this was initialized with.
    unsafe fn set_archetype(&mut self, state: &Self::State, archetype: &Archetype, tables: &Tables);

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `table` must be from the [`World`] [`Fetch::init`] was called on. `state` must be the
    /// [Self::State] this was initialized with.
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

/// A fetch that is read only. This must only be implemented for read-only fetches.
pub unsafe trait ReadOnlyFetch {}

impl WorldQuery for Entity {
    type Fetch = EntityFetch;
    type State = EntityState;
}

/// The [`Fetch`] of [`Entity`].
#[derive(Clone)]
pub struct EntityFetch {
    entities: *const Entity,
}

/// SAFETY: access is read only
unsafe impl ReadOnlyFetch for EntityFetch {}

/// The [`FetchState`] of [`Entity`].
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
}

/// The [`FetchState`] of `&T`.
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
        if access.access().has_write(self.component_id) {
            panic!("&{} conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>());
        }
        access.add_read(self.component_id)
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
}

/// The [`Fetch`] of `&mut T`.
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

/// The [`FetchState`] of `&mut T`.
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
        if access.access().has_read(self.component_id) {
            panic!("&mut {} conflicts with a previous access in this query. Mutable component access must be unique.",
                std::any::type_name::<T>());
        }
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
                        component_ticks: &mut *(&*self.table_ticks.add(table_row)).get(),
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
                component_ticks: &mut *(&*self.table_ticks.add(table_row)).get(),
                change_tick: self.change_tick,
                last_change_tick: self.last_change_tick,
            },
        }
    }
}

impl<T: WorldQuery> WorldQuery for Option<T> {
    type Fetch = OptionFetch<T::Fetch>;
    type State = OptionState<T::State>;
}

/// The [`Fetch`] of `Option<T>`.
#[derive(Clone)]
pub struct OptionFetch<T> {
    fetch: T,
    matches: bool,
}

/// SAFETY: OptionFetch is read only because T is read only
unsafe impl<T: ReadOnlyFetch> ReadOnlyFetch for OptionFetch<T> {}

/// The [`FetchState`] of `Option<T>`.
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
                .update_archetype_component_access(archetype, access)
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
/// # print_moving_objects_system.system();
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
}

/// The [`FetchState`] of [`ChangeTrackers`].
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
        if access.access().has_write(self.component_id) {
            panic!("ChangeTrackers<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>());
        }
        access.add_read(self.component_id)
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
                    component_ticks: (&*self.table_ticks.add(table_row)).clone(),
                    marker: PhantomData,
                    last_change_tick: self.last_change_tick,
                    change_tick: self.change_tick,
                }
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                ChangeTrackers {
                    component_ticks: (&*self.sparse_set).get_ticks(entity).cloned().unwrap(),
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
            component_ticks: (&*self.table_ticks.add(table_row)).clone(),
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
        }

        /// SAFETY: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyFetch),*> ReadOnlyFetch for ($($name,)*) {}

    };
}

all_tuples!(impl_tuple_fetch, 0, 15, F, S);
