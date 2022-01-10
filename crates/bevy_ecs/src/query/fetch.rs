use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    change_detection::Ticks,
    component::{Component, ComponentId, ComponentStorage, ComponentTicks, StorageType},
    entity::Entity,
    ptr::UnsafeCellDeref,
    query::{debug_checked_unreachable, Access, FilteredAccess},
    storage::{ComponentSparseSet, Table, Tables},
    world::{Mut, World},
};
use bevy_ecs_macros::all_tuples;
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
pub trait WorldQuery {
    type State: FetchState + for<'w, 's> FetchInit<'w, 's>;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: QueryItem<'wlong, 'slong, Self>,
    ) -> QueryItem<'wshort, 'sshort, Self>;
}

pub type QueryFetch<'w, 's, Q> = <<Q as WorldQuery>::State as FetchInit<'w, 's>>::Fetch;
pub type QueryItem<'w, 's, Q> = <<Q as WorldQuery>::State as FetchInit<'w, 's>>::Item;
pub type ROQueryFetch<'w, 's, Q> = <<Q as WorldQuery>::State as FetchInit<'w, 's>>::ReadOnlyFetch;
pub type ROQueryItem<'w, 's, Q> = <<Q as WorldQuery>::State as FetchInit<'w, 's>>::ReadOnlyItem;

pub trait FetchInit<'world, 'state>: FetchState {
    type Fetch: Fetch<'world, 'state, State = Self, Item = Self::Item>;
    type Item;

    type ReadOnlyFetch: Fetch<'world, 'state, State = Self, Item = Self::ReadOnlyItem>;
    type ReadOnlyItem;
}

pub trait Fetch<'world, 'state>: Sized {
    type Item;
    type State: FetchState;

    /// Returns true if (and only if) every table of every archetype matched by this fetch contains
    /// all of the matched components. This is used to select a more efficient "table iterator"
    /// for "dense" queries. If this returns true, [`Fetch::set_table`] and [`Fetch::table_fetch`]
    /// will be called for iterators. If this returns false, [`Fetch::set_archetype`] and
    /// [`Fetch::archetype_fetch`] will be called for iterators.
    const IS_DENSE: bool;

    /// Creates a new instance of this fetch.
    ///
    /// # Safety
    ///
    /// `state` must have been initialized (via [FetchState::init]) using the same `world` passed in
    /// to this function.
    unsafe fn init(
        world: &'world World,
        state: &'state Self::State,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self;

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `archetype` and `tables` must be from the [`World`] [`Fetch::init`] was called on. `state` must
    /// be the [Self::State] this was initialized with.
    unsafe fn set_archetype(
        &mut self,
        state: &'state Self::State,
        archetype: &'world Archetype,
        tables: &'world Tables,
    );

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `table` must be from the [`World`] [`Fetch::init`] was called on. `state` must be the
    /// [Self::State] this was initialized with.
    unsafe fn set_table(&mut self, state: &'state Self::State, table: &'world Table);

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
pub unsafe trait ReadOnlyFetch<'w, 's>: Fetch<'w, 's> {}

impl WorldQuery for Entity {
    type State = EntityState;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: QueryItem<'wlong, 'slong, Self>,
    ) -> QueryItem<'wshort, 'sshort, Self> {
        item
    }
}

/// The [`Fetch`] of [`Entity`].
#[derive(Clone)]
pub struct EntityFetch<'w> {
    entities: Option<&'w [Entity]>,
}

/// SAFETY: access is read only
unsafe impl<'w> ReadOnlyFetch<'w, '_> for EntityFetch<'w> {}

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

impl<'w> FetchInit<'w, '_> for EntityState {
    type Fetch = EntityFetch<'w>;
    type Item = Entity;
    type ReadOnlyFetch = EntityFetch<'w>;
    type ReadOnlyItem = Entity;
}

impl<'w, 's> Fetch<'w, 's> for EntityFetch<'w> {
    type Item = Entity;
    type State = EntityState;

    const IS_DENSE: bool = true;

    unsafe fn init(
        _world: &'w World,
        _state: &'s EntityState,
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
        self.entities = Some(archetype.entities());
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, table: &'w Table) {
        self.entities = Some(table.entities());
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let entities = self.entities.unwrap_or_else(|| debug_checked_unreachable());
        debug_assert!(table_row < entities.len());
        *entities.get_unchecked(table_row)
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        let entities = self.entities.unwrap_or_else(|| debug_checked_unreachable());
        debug_assert!(archetype_index < entities.len());
        *entities.get_unchecked(archetype_index)
    }
}

impl<T: Component> WorldQuery for &T {
    type State = ReadState<T>;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: QueryItem<'wlong, 'slong, Self>,
    ) -> QueryItem<'wshort, 'sshort, Self> {
        item
    }
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
pub struct ReadFetch<'w, T> {
    // T::Storage = TableStorage
    table_components: Option<&'w [UnsafeCell<T>]>,
    entity_table_rows: Option<&'w [usize]>,
    // T::Storage = SparseStorage
    entities: Option<&'w [Entity]>,
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
unsafe impl<'w, T: Component> ReadOnlyFetch<'w, '_> for ReadFetch<'w, T> {}

impl<'w, T: Component> FetchInit<'w, '_> for ReadState<T> {
    type Fetch = ReadFetch<'w, T>;
    type Item = &'w T;
    type ReadOnlyFetch = ReadFetch<'w, T>;
    type ReadOnlyItem = &'w T;
}

impl<'w, 's, T: Component> Fetch<'w, 's> for ReadFetch<'w, T> {
    type Item = &'w T;
    type State = ReadState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &'w World,
        state: &'s ReadState<T>,
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
                self.entity_table_rows = Some(archetype.entity_table_rows());
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = Some(column.get_data_slice());
            }
            StorageType::SparseSet => self.entities = Some(archetype.entities()),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        self.table_components = Some(
            table
                .get_column(state.component_id)
                .unwrap()
                .get_data_slice(),
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
                debug_assert!(archetype_index < entity_table_rows.len());
                let table_row = *entity_table_rows.get_unchecked(archetype_index);
                debug_assert!(table_row < table_components.len());
                table_components.get_unchecked(table_row).deref_mut()
            }
            StorageType::SparseSet => {
                let (entities, sparse_set) = self
                    .entities
                    .zip(self.sparse_set)
                    .unwrap_or_else(|| debug_checked_unreachable());
                debug_assert!(archetype_index < entities.len());
                let entity = *entities.get_unchecked(archetype_index);
                sparse_set.get(entity).unwrap().deref::<T>()
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let components = self
            .table_components
            .unwrap_or_else(|| debug_checked_unreachable());
        debug_assert!(table_row < components.len());
        components.get_unchecked(table_row).deref_mut()
    }
}

impl<T: Component> WorldQuery for &mut T {
    type State = WriteState<T>;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: QueryItem<'wlong, 'slong, Self>,
    ) -> QueryItem<'wshort, 'sshort, Self> {
        item
    }
}

/// The [`Fetch`] of `&mut T`.
pub struct WriteFetch<'w, T> {
    table_components: Option<&'w [UnsafeCell<T>]>,
    table_ticks: Option<&'w [UnsafeCell<ComponentTicks>]>,
    entities: Option<&'w [Entity]>,
    entity_table_rows: Option<&'w [usize]>,
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

/// The [`ReadOnlyFetch`] of `&mut T`.
pub struct ReadOnlyWriteFetch<'w, T> {
    table_components: Option<&'w [UnsafeCell<T>]>,
    entities: Option<&'w [Entity]>,
    entity_table_rows: Option<&'w [usize]>,
    sparse_set: Option<&'w ComponentSparseSet>,
}

/// SAFETY: access is read only
unsafe impl<'w, T: Component> ReadOnlyFetch<'w, '_> for ReadOnlyWriteFetch<'w, T> {}

impl<T> Clone for ReadOnlyWriteFetch<'_, T> {
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

impl<'w, T: Component> FetchInit<'w, '_> for WriteState<T> {
    type Fetch = WriteFetch<'w, T>;
    type Item = Mut<'w, T>;

    type ReadOnlyFetch = ReadOnlyWriteFetch<'w, T>;
    type ReadOnlyItem = &'w T;
}

impl<'w, 's, T: Component> Fetch<'w, 's> for WriteFetch<'w, T> {
    type Item = Mut<'w, T>;
    type State = WriteState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &'w World,
        state: &'s WriteState<T>,
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
                self.entity_table_rows = Some(archetype.entity_table_rows());
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = Some(column.get_data_slice());
                self.table_ticks = Some(column.get_ticks());
            }
            StorageType::SparseSet => self.entities = Some(archetype.entities()),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        let column = table.get_column(state.component_id).unwrap();
        self.table_components = Some(column.get_data_slice());
        self.table_ticks = Some(column.get_ticks());
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let (entity_table_rows, (table_components, table_ticks)) = self
                    .entity_table_rows
                    .zip(self.table_components.zip(self.table_ticks))
                    .unwrap_or_else(|| debug_checked_unreachable());
                debug_assert!(archetype_index < entity_table_rows.len());
                let table_row = *entity_table_rows.get_unchecked(archetype_index);
                debug_assert!(table_row < table_components.len());
                Mut {
                    value: table_components.get_unchecked(table_row).deref_mut(),
                    ticks: Ticks {
                        component_ticks: table_ticks.get_unchecked(table_row).deref_mut(),
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
                debug_assert!(archetype_index < entities.len());
                let entity = *entities.get_unchecked(archetype_index);
                let (component, component_ticks) = sparse_set.get_with_ticks(entity).unwrap();
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
        debug_assert!(table_row < table_components.len());
        Mut {
            value: table_components.get_unchecked(table_row).deref_mut(),
            ticks: Ticks {
                component_ticks: table_ticks.get_unchecked(table_row).deref_mut(),
                change_tick: self.change_tick,
                last_change_tick: self.last_change_tick,
            },
        }
    }
}

impl<'w, 's, T: Component> Fetch<'w, 's> for ReadOnlyWriteFetch<'w, T> {
    type Item = &'w T;
    type State = WriteState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &'w World,
        state: &'s WriteState<T>,
        _last_change_tick: u32,
        _change_tick: u32,
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
                self.entity_table_rows = Some(archetype.entity_table_rows());
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_components = Some(column.get_data_slice());
            }
            StorageType::SparseSet => self.entities = Some(archetype.entities()),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        self.table_components = Some(
            table
                .get_column(state.component_id)
                .unwrap()
                .get_data_slice(),
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
                debug_assert!(archetype_index < entity_table_rows.len());
                let table_row = *entity_table_rows.get_unchecked(archetype_index);
                debug_assert!(table_row < table_components.len());
                table_components.get_unchecked(table_row).deref_mut()
            }
            StorageType::SparseSet => {
                let (entities, sparse_set) = self
                    .entities
                    .zip(self.sparse_set)
                    .unwrap_or_else(|| debug_checked_unreachable());
                debug_assert!(archetype_index < entities.len());
                let entity = *entities.get_unchecked(archetype_index);
                sparse_set.get(entity).unwrap().deref::<T>()
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let components = self
            .table_components
            .unwrap_or_else(|| debug_checked_unreachable());
        debug_assert!(table_row < components.len());
        components.get_unchecked(table_row).deref_mut()
    }
}

impl<T: WorldQuery> WorldQuery for Option<T> {
    type State = OptionState<T::State>;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: QueryItem<'wlong, 'slong, Self>,
    ) -> QueryItem<'wshort, 'sshort, Self> {
        item.map(T::shrink)
    }
}

/// The [`Fetch`] of `Option<T>`.
#[derive(Clone)]
pub struct OptionFetch<T> {
    fetch: T,
    matches: bool,
}

/// SAFETY: OptionFetch is read only because T is read only
unsafe impl<'w, 's, T: ReadOnlyFetch<'w, 's>> ReadOnlyFetch<'w, 's> for OptionFetch<T> {}

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

impl<'w, 's, T: FetchState + FetchInit<'w, 's>> FetchInit<'w, 's> for OptionState<T> {
    type Fetch = OptionFetch<T::Fetch>;
    type Item = Option<T::Item>;
    type ReadOnlyFetch = OptionFetch<T::ReadOnlyFetch>;
    type ReadOnlyItem = Option<T::ReadOnlyItem>;
}

impl<'w, 's, T: Fetch<'w, 's>> Fetch<'w, 's> for OptionFetch<T> {
    type Item = Option<T::Item>;
    type State = OptionState<T::State>;

    const IS_DENSE: bool = T::IS_DENSE;

    unsafe fn init(
        world: &'w World,
        state: &'s OptionState<T::State>,
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
        state: &'s Self::State,
        archetype: &'w Archetype,
        tables: &'w Tables,
    ) {
        self.matches = state.state.matches_archetype(archetype);
        if self.matches {
            self.fetch.set_archetype(&state.state, archetype, tables);
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &'s Self::State, table: &'w Table) {
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
    type State = ChangeTrackersState<T>;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: QueryItem<'wlong, 'slong, Self>,
    ) -> QueryItem<'wshort, 'sshort, Self> {
        item
    }
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
pub struct ChangeTrackersFetch<'w, T> {
    table_ticks: Option<&'w [UnsafeCell<ComponentTicks>]>,
    entity_table_rows: Option<&'w [usize]>,
    entities: Option<&'w [Entity]>,
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
unsafe impl<'w, T: Component> ReadOnlyFetch<'w, '_> for ChangeTrackersFetch<'w, T> {}

impl<'w, T: Component> FetchInit<'w, '_> for ChangeTrackersState<T> {
    type Fetch = ChangeTrackersFetch<'w, T>;
    type Item = ChangeTrackers<T>;
    type ReadOnlyFetch = ChangeTrackersFetch<'w, T>;
    type ReadOnlyItem = ChangeTrackers<T>;
}

impl<'w, 's, T: Component> Fetch<'w, 's> for ChangeTrackersFetch<'w, T> {
    type Item = ChangeTrackers<T>;
    type State = ChangeTrackersState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        world: &'w World,
        state: &'s ChangeTrackersState<T>,
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
                self.entity_table_rows = Some(archetype.entity_table_rows());
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id)
                    .unwrap();
                self.table_ticks = Some(column.get_ticks());
            }
            StorageType::SparseSet => self.entities = Some(archetype.entities()),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
        self.table_ticks = Some(table.get_column(state.component_id).unwrap().get_ticks());
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => {
                let entity_table_rows = self
                    .entity_table_rows
                    .unwrap_or_else(|| debug_checked_unreachable());
                debug_assert!(archetype_index < entity_table_rows.len());
                let table_row = *entity_table_rows.get_unchecked(archetype_index);
                ChangeTrackers {
                    component_ticks: {
                        let table_ticks = self
                            .table_ticks
                            .unwrap_or_else(|| debug_checked_unreachable());
                        debug_assert!(table_row < table_ticks.len());
                        table_ticks.get_unchecked(table_row).deref().clone()
                    },
                    marker: PhantomData,
                    last_change_tick: self.last_change_tick,
                    change_tick: self.change_tick,
                }
            }
            StorageType::SparseSet => {
                let entities = self.entities.unwrap_or_else(|| debug_checked_unreachable());
                debug_assert!(archetype_index < entities.len());
                let entity = *entities.get_unchecked(archetype_index);
                ChangeTrackers {
                    component_ticks: self
                        .sparse_set
                        .unwrap_or_else(|| debug_checked_unreachable())
                        .get_ticks(entity)
                        .cloned()
                        .unwrap(),
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
                debug_assert!(table_row < table_ticks.len());
                table_ticks.get_unchecked(table_row).deref().clone()
            },
            marker: PhantomData,
            last_change_tick: self.last_change_tick,
            change_tick: self.change_tick,
        }
    }
}

macro_rules! impl_tuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, 's, $($name: FetchInit<'w, 's>),*> FetchInit<'w, 's> for ($($name,)*) {
            type Fetch = ($($name::Fetch,)*);
            type Item = ($($name::Item,)*);

            type ReadOnlyFetch = ($($name::ReadOnlyFetch,)*);
            type ReadOnlyItem = ($($name::ReadOnlyItem,)*);
        }

        #[allow(non_snake_case)]
        impl<'w, 's, $($name: Fetch<'w, 's>),*> Fetch<'w, 's> for ($($name,)*) {
            type Item = ($($name::Item,)*);
            type State = ($($name::State,)*);

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            #[allow(clippy::unused_unit)]
            unsafe fn init(_world: &'w World, state: &'s Self::State, _last_change_tick: u32, _change_tick: u32) -> Self {
                let ($($name,)*) = state;
                ($($name::init(_world, $name, _last_change_tick, _change_tick),)*)
            }

            #[inline]
            unsafe fn set_archetype(&mut self, _state: &'s Self::State, _archetype: &'w Archetype, _tables: &'w Tables) {
                let ($($name,)*) = self;
                let ($($state,)*) = _state;
                $($name.set_archetype($state, _archetype, _tables);)*
            }

            #[inline]
            unsafe fn set_table(&mut self, _state: &'s Self::State, _table: &'w Table) {
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

        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        impl<$($name: WorldQuery),*> WorldQuery for ($($name,)*) {
            type State = ($($name::State,)*);

            fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
                item: QueryItem<'wlong, 'slong, Self>,
            ) -> QueryItem<'wshort, 'sshort, Self> {
                let ($($name,)*) = item;
                ($(
                    $name::shrink($name),
                )*)
            }
        }

        /// SAFETY: each item in the tuple is read only
        unsafe impl<'w, 's, $($name: ReadOnlyFetch<'w, 's>),*> ReadOnlyFetch<'w, 's> for ($($name,)*) {}

    };
}

all_tuples!(impl_tuple_fetch, 0, 15, F, S);

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
    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {}

    #[inline(always)]
    unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {}
}
