use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeComponentInfo},
    change_detection::Ticks,
    component::{Component, ComponentDescriptor, ComponentId, ComponentTicks, StorageType},
    entity::Entity,
    query::{Access, FilteredAccess},
    storage::{Column, ComponentSparseSet, SparseSets, Table, Tables},
    world::{Mut, World},
};
use bevy_ecs_macros::all_tuples;
use bevy_utils::{HashMap, StableHashMap};
use smallvec::SmallVec;
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
    type Fetch: for<'world, 'state> Fetch<
        'world,
        'state,
        State = Self::State,
        TargetFilter = <Self::State as FetchState>::TargetFilter,
    >;
    type State: FetchState;
}

pub trait Fetch<'world, 'state>: Sized {
    type Item;
    type State: FetchState<TargetFilter = Self::TargetFilter>;
    type TargetFilter: Clone + std::hash::Hash + PartialEq + Eq + Default + Send + Sync + 'static;

    /// Creates a new instance of this fetch.
    ///
    /// # Safety
    ///
    /// `state` must have been initialized (via [FetchState::init]) using the same `world` passed in
    /// to this function.
    unsafe fn init(
        world: &World,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self;

    /// Returns true if (and only if) every table of every archetype matched by this Fetch contains
    /// all of the matched components. This is used to select a more efficient "table iterator"
    /// for "dense" queries. If this returns true, [`Fetch::set_table`] and [`Fetch::table_fetch`]
    /// will be called for iterators. If this returns false, [`Fetch::set_archetype`] and
    /// [`Fetch::archetype_fetch`] will be called for iterators.
    fn is_dense(&self) -> bool;

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `archetype` and `tables` must be from the [`World`] [`Fetch::init`] was called on. `state` must
    /// be the [Self::State] this was initialized with.
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        archetype: &Archetype,
        tables: &Tables,
    );

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`Fetch`].
    ///
    /// # Safety
    ///
    /// `table` must be from the [`World`] [`Fetch::init`] was called on. `state` must be the
    /// [Self::State] this was initialized with.
    unsafe fn set_table(
        &mut self,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        table: &Table,
    );

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
    type TargetFilter: Clone + std::hash::Hash + PartialEq + Eq + Default + Send + Sync + 'static;

    fn init(world: &mut World) -> Self;
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>);
    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    );
    fn matches_archetype(&self, archetype: &Archetype, target_filter: &Self::TargetFilter) -> bool;
    fn matches_table(&self, table: &Table, target_filter: &Self::TargetFilter) -> bool;
    fn deduplicate_targets(target_filter: &mut Self::TargetFilter);
}

/// A fetch that is read only. This must only be implemented for read-only fetches.
pub unsafe trait ReadOnlyFetch {}

impl WorldQuery for Entity {
    type Fetch = EntityFetch;
    type State = EntityState;
}

/// The [`Fetch`] of [`Entity`].
pub struct EntityFetch {
    entities: *const Entity,
}

/// SAFETY: access is read only
unsafe impl ReadOnlyFetch for EntityFetch {}

/// The [`FetchState`] of [`Entity`].
pub struct EntityState;

// SAFETY: no component or archetype access
unsafe impl FetchState for EntityState {
    type TargetFilter = ();

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
    fn matches_archetype(&self, _archetype: &Archetype, _: &Self::TargetFilter) -> bool {
        true
    }

    #[inline]
    fn matches_table(&self, _table: &Table, _: &Self::TargetFilter) -> bool {
        true
    }

    fn deduplicate_targets(_: &mut Self::TargetFilter) {}
}

impl<'w, 's> Fetch<'w, 's> for EntityFetch {
    type Item = Entity;
    type State = EntityState;
    type TargetFilter = ();

    #[inline]
    fn is_dense(&self) -> bool {
        true
    }

    unsafe fn init(
        _world: &World,
        _state: &Self::State,
        _: &Self::TargetFilter,
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
        _: &Self::TargetFilter,
        archetype: &Archetype,
        _tables: &Tables,
    ) {
        self.entities = archetype.entities().as_ptr();
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, _: &Self::TargetFilter, table: &Table) {
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
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// read
unsafe impl<T: Component> FetchState for ReadState<T> {
    type TargetFilter = ();

    fn init(world: &mut World) -> Self {
        let component_info = world.components.component_info_or_insert::<T>();
        ReadState {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
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
            archetype.get_archetype_component_id(self.component_id, None)
        {
            access.add_read(archetype_component_id);
        }
    }

    fn matches_archetype(&self, archetype: &Archetype, _: &Self::TargetFilter) -> bool {
        archetype.contains(self.component_id, None)
    }

    fn matches_table(&self, table: &Table, _: &Self::TargetFilter) -> bool {
        table.has_column(self.component_id, None)
    }

    fn deduplicate_targets(_: &mut Self::TargetFilter) {}
}

/// The [`Fetch`] of `&T`.
pub struct ReadFetch<T> {
    storage_type: StorageType,
    table_components: NonNull<T>,
    entity_table_rows: *const usize,
    entities: *const Entity,
    sparse_set: *const ComponentSparseSet,
}

impl<T> Clone for ReadFetch<T> {
    fn clone(&self) -> Self {
        Self {
            storage_type: self.storage_type,
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
    type TargetFilter = ();

    #[inline]
    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn init(
        world: &World,
        state: &Self::State,
        _: &Self::TargetFilter,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        let mut value = Self {
            storage_type: state.storage_type,
            table_components: NonNull::dangling(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
        };
        if state.storage_type == StorageType::SparseSet {
            value.sparse_set = world
                .storages()
                .sparse_sets
                .get(state.component_id, None)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        _: &Self::TargetFilter,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match state.storage_type {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id, None)
                    .unwrap();
                self.table_components = column.get_data_ptr().cast::<T>();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, _: &Self::TargetFilter, table: &Table) {
        self.table_components = table
            .get_column(state.component_id, None)
            .unwrap()
            .get_data_ptr()
            .cast::<T>();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
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
    storage_type: StorageType,
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
            storage_type: self.storage_type,
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
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// written
unsafe impl<T: Component> FetchState for WriteState<T> {
    type TargetFilter = ();

    fn init(world: &mut World) -> Self {
        let component_info = world.components.component_info_or_insert::<T>();
        WriteState {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
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
            archetype.get_archetype_component_id(self.component_id, None)
        {
            access.add_write(archetype_component_id);
        }
    }

    fn matches_archetype(&self, archetype: &Archetype, _: &Self::TargetFilter) -> bool {
        archetype.contains(self.component_id, None)
    }

    fn matches_table(&self, table: &Table, _: &Self::TargetFilter) -> bool {
        table.has_column(self.component_id, None)
    }

    fn deduplicate_targets(_: &mut Self::TargetFilter) {}
}

impl<'w, 's, T: Component> Fetch<'w, 's> for WriteFetch<T> {
    type Item = Mut<'w, T>;
    type State = WriteState<T>;
    type TargetFilter = ();

    #[inline]
    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn init(
        world: &World,
        state: &Self::State,
        _: &Self::TargetFilter,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        let mut value = Self {
            storage_type: state.storage_type,
            table_components: NonNull::dangling(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
            table_ticks: ptr::null::<UnsafeCell<ComponentTicks>>(),
            last_change_tick,
            change_tick,
        };
        if state.storage_type == StorageType::SparseSet {
            value.sparse_set = world
                .storages()
                .sparse_sets
                .get(state.component_id, None)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        _: &Self::TargetFilter,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match state.storage_type {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id, None)
                    .unwrap();
                self.table_components = column.get_data_ptr().cast::<T>();
                self.table_ticks = column.get_ticks_ptr();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, _: &Self::TargetFilter, table: &Table) {
        let column = table.get_column(state.component_id, None).unwrap();
        self.table_components = column.get_data_ptr().cast::<T>();
        self.table_ticks = column.get_ticks_ptr();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
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

#[derive(Debug)]
pub enum Either<T, U> {
    T(T),
    U(U),
}

pub struct Relation<T: Component>(std::marker::PhantomData<T>, [u8]);

impl<T: Component> WorldQuery for &Relation<T> {
    type Fetch = ReadRelationFetch<T>;
    type State = ReadRelationState<T>;
}

pub struct ReadRelationState<T> {
    p: PhantomData<T>,
    component_id: ComponentId,
    storage_type: StorageType,
}

unsafe impl<T: Component> FetchState for ReadRelationState<T> {
    type TargetFilter = smallvec::SmallVec<[Entity; 4]>;

    fn init(world: &mut World) -> Self {
        let component_info =
            world
                .components
                .component_info_or_insert_from(ComponentDescriptor::new_targeted::<T>(
                    StorageType::Table,
                ));

        Self {
            p: PhantomData,
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        if access.access().has_write(self.component_id) {
            panic!("&Relation<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                std::any::type_name::<T>());
        }
        access.add_read(self.component_id);
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if self.matches_archetype(archetype, &Default::default()) {
            // FIXME(Relationships): make `ArchetypeComponentId` work like `ComponentId` and not be
            // a fresh ID for every target I think? Need to investigate more what `ArchetypeComponentId`
            // is actually used for and if that is even possible to do :)
            let targets = archetype.relations.get(self.component_id).unwrap();
            for id in targets.values() {
                access.add_read(id.archetype_component_id);
            }
        }
    }

    fn matches_archetype(
        &self,
        archetype: &Archetype,
        target_filter: &SmallVec<[Entity; 4]>,
    ) -> bool {
        if archetype.relations.get(self.component_id).is_none() {
            return false;
        }
        target_filter
            .iter()
            .all(|target| archetype.contains(self.component_id, Some(*target)))
    }

    fn matches_table(&self, table: &Table, target_filter: &SmallVec<[Entity; 4]>) -> bool {
        if table
            .targeted_component_columns
            .get(self.component_id)
            .is_none()
        {
            return false;
        }
        target_filter
            .iter()
            .all(|target| table.has_column(self.component_id, Some(*target)))
    }

    fn deduplicate_targets(target_filter: &mut Self::TargetFilter) {
        target_filter.sort();
        target_filter.dedup();
    }
}

pub struct ReadRelationFetch<T> {
    component_id: ComponentId,
    target_filter_ptr: *const [Entity],

    table_ptr: *const Table,
    archetype_ptr: *const Archetype,
    entity_table_rows: *const [usize],
    entities: *const [Entity],
    sparse_sets: *const SparseSets,

    storage_type: StorageType,
    p: PhantomData<T>,
}

unsafe impl<T: Component> ReadOnlyFetch for ReadRelationFetch<T> {}

#[derive(Debug)]
pub struct TableRelationAccess<'w, 's, T: Component> {
    current_idx: usize,
    columns: &'w StableHashMap<Entity, Column>,
    iter:
        Either<std::collections::hash_map::Keys<'w, Entity, Column>, std::slice::Iter<'s, Entity>>,
    p: PhantomData<&'w T>,
}

#[derive(Debug)]
pub struct SparseRelationAccess<'w, 's, T: Component> {
    current_entity: Entity,
    sparse_sets: &'w HashMap<Entity, ComponentSparseSet>,
    iter: Either<
        std::collections::hash_map::Keys<'w, Entity, ArchetypeComponentInfo>,
        std::slice::Iter<'s, Entity>,
    >,
    p: PhantomData<&'w T>,
}

// We split these out to separate structs so that the fields are private
#[derive(Debug)]
pub enum RelationAccess<'w, 's, T: Component> {
    Table(TableRelationAccess<'w, 's, T>),
    Sparse(SparseRelationAccess<'w, 's, T>),
}

impl<'w, 's, T: Component> RelationAccess<'w, 's, T> {
    pub fn single(&mut self) -> <Self as Iterator>::Item {
        let ret = self.next().unwrap();
        assert!(matches!(self.next(), None));
        ret
    }
}

impl<'w, 's, T: Component> Iterator for RelationAccess<'w, 's, T> {
    type Item = (Entity, &'w T);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Table(TableRelationAccess {
                current_idx,
                columns,
                iter,
                ..
            }) => unsafe {
                let target = match iter {
                    Either::T(target_iter) => target_iter.next()?,
                    Either::U(target_iter) => target_iter.next()?,
                };
                // SAFETY: we remove duplicate target filters in `ReadRelationState::deduplicate_targets`
                // so this will not lead to aliasing borrows if users insert two identical target filters
                let col = columns.get(target).unwrap();
                let ptr = col.get_data_unchecked(*current_idx) as *mut T;
                Some((*target, &*ptr))
            },
            Self::Sparse(SparseRelationAccess {
                current_entity,
                sparse_sets,
                iter,
                ..
            }) => unsafe {
                let target = match iter {
                    Either::T(target_iter) => target_iter.next()?,
                    Either::U(target_iter) => target_iter.next()?,
                };
                // SAFETY: we remove duplicate target filters in `ReadRelationState::deduplicate_targets`
                // so this will not lead to aliasing borrows if users insert two identical target filters
                let set = sparse_sets.get(target).unwrap();
                let ptr = set.get(*current_entity).unwrap() as *mut T;
                Some((*target, &*ptr))
            },
        }
    }
}

impl<'w, 's, T: Component> Fetch<'w, 's> for ReadRelationFetch<T> {
    type Item = RelationAccess<'w, 's, T>;
    type State = ReadRelationState<T>;
    type TargetFilter = SmallVec<[Entity; 4]>;

    unsafe fn init(
        world: &World,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        Self {
            component_id: state.component_id,
            target_filter_ptr: target_filter.as_slice(),

            table_ptr: 0x0 as _,
            archetype_ptr: 0x0 as _,
            entity_table_rows: &[],
            entities: &[],
            sparse_sets: &world.storages.sparse_sets,

            storage_type: state.storage_type,
            p: PhantomData,
        }
    }

    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        _: &Self::TargetFilter,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        self.entity_table_rows = archetype.entity_table_rows();
        self.archetype_ptr = archetype;
        self.table_ptr = &tables[archetype.table_id()];
        self.entities = archetype.entities();
    }

    unsafe fn set_table(&mut self, _state: &Self::State, _: &Self::TargetFilter, table: &Table) {
        self.table_ptr = table;
    }

    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
            StorageType::Table => {
                let table_row = (&*self.entity_table_rows)[archetype_index];
                self.table_fetch(table_row)
            }
            StorageType::SparseSet => {
                let target_filters = &*self.target_filter_ptr;
                let sparse_sets = &*self.sparse_sets;
                let archetype = &*self.archetype_ptr;

                let iter = match target_filters.len() {
                    0 => Either::T(archetype.relations.get(self.component_id).unwrap().keys()),
                    _ => Either::U(target_filters.iter()),
                };

                RelationAccess::Sparse(SparseRelationAccess {
                    current_entity: (&*self.entities)[archetype_index],
                    sparse_sets: sparse_sets
                        .get_sets_of_component_id(self.component_id)
                        .unwrap(),
                    iter,
                    p: PhantomData,
                })
            }
        }
    }

    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        // FIXME(Relationships) store a ptr to `table.relation_columns.get(self.component_id)` instead of this
        let table = &*self.table_ptr;

        let target_filters = &*self.target_filter_ptr;
        let iter = match target_filters.len() {
            0 => Either::T(
                table
                    .targeted_component_columns
                    .get(self.component_id)
                    .unwrap()
                    .keys(),
            ),
            _ => Either::U(target_filters.iter()),
        };

        RelationAccess::Table(TableRelationAccess {
            columns: table
                .targeted_component_columns
                .get(self.component_id)
                .unwrap(),
            current_idx: table_row,
            iter,
            p: PhantomData,
        })
    }
}

impl<T: Component> WorldQuery for &mut Relation<T> {
    type Fetch = WriteRelationFetch<T>;
    type State = WriteRelationState<T>;
}

pub struct WriteRelationState<T> {
    p: PhantomData<T>,
    component_id: ComponentId,
    storage_type: StorageType,
}

unsafe impl<T: Component> FetchState for WriteRelationState<T> {
    type TargetFilter = smallvec::SmallVec<[Entity; 4]>;

    fn init(world: &mut World) -> Self {
        let component_info =
            world
                .components
                .component_info_or_insert_from(ComponentDescriptor::new_targeted::<T>(
                    StorageType::Table,
                ));

        Self {
            p: PhantomData,
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        if access.access().has_read(self.component_id) {
            panic!("&mut Relation<{}> conflicts with a previous access in this query. Mutable access must be exclusive.",
                std::any::type_name::<T>());
        }
        access.add_write(self.component_id);
    }

    fn update_archetype_component_access(
        &self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        if self.matches_archetype(archetype, &Default::default()) {
            let targets = archetype.relations.get(self.component_id).unwrap();
            for id in targets.values() {
                access.add_write(id.archetype_component_id);
            }
        }
    }

    fn matches_archetype(
        &self,
        archetype: &Archetype,
        target_filter: &SmallVec<[Entity; 4]>,
    ) -> bool {
        if archetype.relations.get(self.component_id).is_none() {
            return false;
        }
        target_filter
            .iter()
            .all(|target| archetype.contains(self.component_id, Some(*target)))
    }

    fn matches_table(&self, table: &Table, target_filter: &SmallVec<[Entity; 4]>) -> bool {
        if table
            .targeted_component_columns
            .get(self.component_id)
            .is_none()
        {
            return false;
        }
        target_filter
            .iter()
            .all(|target| table.has_column(self.component_id, Some(*target)))
    }

    fn deduplicate_targets(target_filter: &mut Self::TargetFilter) {
        target_filter.sort();
        target_filter.dedup();
    }
}

pub struct WriteRelationFetch<T> {
    component_id: ComponentId,
    target_filter_ptr: *const [Entity],
    last_change_tick: u32,
    change_tick: u32,

    table_ptr: *const Table,
    archetype_ptr: *const Archetype,
    entity_table_rows: *const [usize],
    entities: *const [Entity],
    sparse_sets: *const SparseSets,

    storage_type: StorageType,
    p: PhantomData<T>,
}

#[derive(Debug)]
pub struct RelationAccessMut<'w, 's, T: Component> {
    access: RelationAccess<'w, 's, T>,
    change_tick: u32,
    last_change_tick: u32,
}

impl<'w, 's, T: Component> RelationAccessMut<'w, 's, T> {
    pub fn single(&mut self) -> <Self as Iterator>::Item {
        let ret = self.next().unwrap();
        assert!(matches!(self.next(), None));
        ret
    }
}

impl<'w, 's, T: Component> Iterator for RelationAccessMut<'w, 's, T> {
    type Item = (Entity, Mut<'w, T>);

    fn next(&mut self) -> Option<Self::Item> {
        let (target, ptr, ticks) = match &mut self.access {
            RelationAccess::Table(TableRelationAccess {
                current_idx,
                columns,
                iter,
                ..
            }) => unsafe {
                let target = match iter {
                    Either::T(target_iter) => target_iter.next()?,
                    Either::U(target_iter) => target_iter.next()?,
                };
                // SAFETY: we remove duplicate target filters in `WriteRelationState::deduplicate_targets`
                // so this will not lead to aliasing borrows if users insert two identical target filters
                let col = columns.get(target).unwrap();
                let ptr = col.get_data_unchecked(*current_idx) as *mut T;
                let ticks = col.get_ticks_mut_ptr_unchecked(*current_idx);
                (target, ptr, ticks)
            },
            RelationAccess::Sparse(SparseRelationAccess {
                current_entity,
                sparse_sets,
                iter,
                ..
            }) => unsafe {
                let target = match iter {
                    Either::T(target_iter) => target_iter.next()?,
                    Either::U(target_iter) => target_iter.next()?,
                };
                // SAFETY: we remove duplicate target filters in `WriteRelationState::deduplicate_targets`
                // so this will not lead to aliasing borrows if users insert two identical target filters
                let set = sparse_sets.get(target).unwrap();
                let (ptr, ticks) = set.get_with_ticks(*current_entity).unwrap();
                let ptr = ptr as *mut T;
                (target, ptr, ticks)
            },
        };

        Some((
            *target,
            Mut {
                value: unsafe { &mut *ptr },
                ticks: Ticks {
                    component_ticks: unsafe { &mut *ticks },
                    last_change_tick: self.last_change_tick,
                    change_tick: self.change_tick,
                },
            },
        ))
    }
}

impl<'w, 's, T: Component> Fetch<'w, 's> for WriteRelationFetch<T> {
    type Item = RelationAccessMut<'w, 's, T>;
    type State = WriteRelationState<T>;
    type TargetFilter = SmallVec<[Entity; 4]>;

    unsafe fn init(
        world: &World,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            component_id: state.component_id,
            target_filter_ptr: target_filter.as_slice(),
            last_change_tick,
            change_tick,

            table_ptr: 0x0 as _,
            archetype_ptr: 0x0 as _,
            entity_table_rows: &[],
            entities: &[],
            sparse_sets: &world.storages.sparse_sets,

            storage_type: state.storage_type,
            p: PhantomData,
        }
    }

    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        _: &Self::TargetFilter,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        self.entity_table_rows = archetype.entity_table_rows();
        self.archetype_ptr = archetype;
        self.table_ptr = &tables[archetype.table_id()];
        self.entities = archetype.entities();
    }

    unsafe fn set_table(&mut self, _state: &Self::State, _: &Self::TargetFilter, table: &Table) {
        self.table_ptr = table;
    }

    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
            StorageType::Table => {
                let table_row = (&*self.entity_table_rows)[archetype_index];
                self.table_fetch(table_row)
            }
            StorageType::SparseSet => {
                let target_filters = &*self.target_filter_ptr;
                let sparse_sets = &*self.sparse_sets;
                let archetype = &*self.archetype_ptr;

                let iter = match target_filters.len() {
                    0 => Either::T(archetype.relations.get(self.component_id).unwrap().keys()),
                    _ => Either::U(target_filters.iter()),
                };

                RelationAccessMut {
                    access: RelationAccess::Sparse(SparseRelationAccess {
                        current_entity: (&*self.entities)[archetype_index],
                        sparse_sets: sparse_sets
                            .get_sets_of_component_id(self.component_id)
                            .unwrap(),
                        iter,
                        p: PhantomData,
                    }),
                    change_tick: self.change_tick,
                    last_change_tick: self.last_change_tick,
                }
            }
        }
    }

    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        let table = &*self.table_ptr;

        let target_filters = &*self.target_filter_ptr;
        let iter = match target_filters.len() {
            0 => Either::T(
                table
                    .targeted_component_columns
                    .get(self.component_id)
                    .unwrap()
                    .keys(),
            ),
            _ => Either::U(target_filters.iter()),
        };

        RelationAccessMut {
            access: RelationAccess::Table(TableRelationAccess {
                columns: table
                    .targeted_component_columns
                    .get(self.component_id)
                    .unwrap(),
                current_idx: table_row,
                iter,
                p: PhantomData,
            }),
            change_tick: self.change_tick,
            last_change_tick: self.last_change_tick,
        }
    }
}

impl<T: WorldQuery> WorldQuery for Option<T> {
    type Fetch = OptionFetch<T::Fetch>;
    type State = OptionState<T::State>;
}

/// The [`Fetch`] of `Option<T>`.
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
    type TargetFilter = T::TargetFilter;

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
        if self.state.matches_archetype(archetype, &Default::default()) {
            self.state
                .update_archetype_component_access(archetype, access)
        }
    }

    fn matches_archetype(&self, _archetype: &Archetype, _: &Self::TargetFilter) -> bool {
        true
    }

    fn matches_table(&self, _table: &Table, _: &Self::TargetFilter) -> bool {
        true
    }

    fn deduplicate_targets(target_filter: &mut Self::TargetFilter) {
        T::deduplicate_targets(target_filter);
    }
}

impl<'w, 's, T: Fetch<'w, 's>> Fetch<'w, 's> for OptionFetch<T> {
    type Item = Option<T::Item>;
    type State = OptionState<T::State>;
    type TargetFilter = T::TargetFilter;

    #[inline]
    fn is_dense(&self) -> bool {
        self.fetch.is_dense()
    }

    unsafe fn init(
        world: &World,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        Self {
            fetch: T::init(
                world,
                &state.state,
                target_filter,
                last_change_tick,
                change_tick,
            ),
            matches: false,
        }
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        self.matches = state.state.matches_archetype(archetype, target_filter);
        if self.matches {
            self.fetch
                .set_archetype(&state.state, target_filter, archetype, tables);
        }
    }

    #[inline]
    unsafe fn set_table(
        &mut self,
        state: &Self::State,
        target_filter: &Self::TargetFilter,
        table: &Table,
    ) {
        self.matches = state.state.matches_table(table, target_filter);
        if self.matches {
            self.fetch.set_table(&state.state, target_filter, table);
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
/// # use bevy_ecs::system::Query;
/// # use bevy_ecs::query::ChangeTrackers;
/// # use bevy_ecs::system::IntoSystem;
/// #
/// # #[derive(Debug)]
/// # struct Name {};
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
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFETY: component access and archetype component access are properly updated to reflect that T is
// read
unsafe impl<T: Component> FetchState for ChangeTrackersState<T> {
    type TargetFilter = ();

    fn init(world: &mut World) -> Self {
        let component_info = world.components.component_info_or_insert::<T>();

        Self {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
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
            archetype.get_archetype_component_id(self.component_id, None)
        {
            access.add_read(archetype_component_id);
        }
    }

    fn matches_archetype(&self, archetype: &Archetype, _: &Self::TargetFilter) -> bool {
        archetype.contains(self.component_id, None)
    }

    fn matches_table(&self, table: &Table, _: &Self::TargetFilter) -> bool {
        table.has_column(self.component_id, None)
    }

    fn deduplicate_targets(_: &mut Self::TargetFilter) {}
}

/// The [`Fetch`] of [`ChangeTrackers`].
pub struct ChangeTrackersFetch<T> {
    storage_type: StorageType,
    table_ticks: *const ComponentTicks,
    entity_table_rows: *const usize,
    entities: *const Entity,
    sparse_set: *const ComponentSparseSet,
    marker: PhantomData<T>,
    last_change_tick: u32,
    change_tick: u32,
}

/// SAFETY: access is read only
unsafe impl<T> ReadOnlyFetch for ChangeTrackersFetch<T> {}

impl<'w, 's, T: Component> Fetch<'w, 's> for ChangeTrackersFetch<T> {
    type Item = ChangeTrackers<T>;
    type State = ChangeTrackersState<T>;
    type TargetFilter = ();

    #[inline]
    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn init(
        world: &World,
        state: &Self::State,
        _: &Self::TargetFilter,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        let mut value = Self {
            storage_type: state.storage_type,
            table_ticks: ptr::null::<ComponentTicks>(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
            marker: PhantomData,
            last_change_tick,
            change_tick,
        };
        if state.storage_type == StorageType::SparseSet {
            value.sparse_set = world
                .storages()
                .sparse_sets
                .get(state.component_id, None)
                .unwrap();
        }
        value
    }

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        state: &Self::State,
        _: &Self::TargetFilter,
        archetype: &Archetype,
        tables: &Tables,
    ) {
        match state.storage_type {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                let column = tables[archetype.table_id()]
                    .get_column(state.component_id, None)
                    .unwrap();
                self.table_ticks = column.get_ticks_const_ptr();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, _: &Self::TargetFilter, table: &Table) {
        self.table_ticks = table
            .get_column(state.component_id, None)
            .unwrap()
            .get_ticks_const_ptr();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
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
    ($(($name: ident, $state: ident, $target_filter: ident)),*) => {
        #[allow(non_snake_case)]
        impl<'w, 's, $($name: Fetch<'w, 's>),*> Fetch<'w, 's> for ($($name,)*) {
            type Item = ($($name::Item,)*);
            type State = ($($name::State,)*);
            type TargetFilter = ($($name::TargetFilter,)*);

            #[allow(clippy::unused_unit)]
            unsafe fn init(_world: &World, state: &Self::State, target_filter: &Self::TargetFilter, _last_change_tick: u32, _change_tick: u32) -> Self {
                let ($($name,)*) = state;
                let ($($target_filter,)*) = target_filter;
                ($($name::init(_world, $name, $target_filter, _last_change_tick, _change_tick),)*)
            }


            #[inline]
            fn is_dense(&self) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.is_dense())*
            }

            #[inline]
            unsafe fn set_archetype(&mut self, _state: &Self::State, target_filter: &Self::TargetFilter, _archetype: &Archetype, _tables: &Tables) {
                let ($($name,)*) = self;
                let ($($state,)*) = _state;
                let ($($target_filter,)*) = target_filter;
                $($name.set_archetype($state, $target_filter, _archetype, _tables);)*
            }

            #[inline]
            unsafe fn set_table(&mut self, _state: &Self::State, _target_filter: &Self::TargetFilter, _table: &Table) {
                let ($($name,)*) = self;
                let ($($state,)*) = _state;
                let ($($target_filter,)*) = _target_filter;
                $($name.set_table($state, $target_filter, _table);)*
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
            type TargetFilter = ($($name::TargetFilter,)*);

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

            fn matches_archetype(&self, _archetype: &Archetype, _target_filter: &Self::TargetFilter) -> bool {
                let ($($name,)*) = self;
                let ($($target_filter,)*) = _target_filter;
                true $(&& $name.matches_archetype(_archetype, $target_filter))*
            }

            fn matches_table(&self, _table: &Table, _target_filter: &Self::TargetFilter) -> bool {
                let ($($name,)*) = self;
                let ($($target_filter,)*) = _target_filter;
                true $(&& $name.matches_table(_table, $target_filter))*
            }

            fn deduplicate_targets(target_filter: &mut Self::TargetFilter) {
                let ($($name,)*) = target_filter;
                $($name::deduplicate_targets($name);)*
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

all_tuples!(impl_tuple_fetch, 0, 11, F, S, R);
