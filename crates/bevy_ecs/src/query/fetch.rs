use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{Component, ComponentFlags, ComponentId, StorageType},
    entity::Entity,
    query::{Access, FilteredAccess},
    storage::{ComponentSparseSet, Table, Tables},
    world::{Mut, World},
};
use bevy_ecs_macros::all_tuples;
use std::{
    marker::PhantomData,
    ptr::{self, NonNull},
};

pub trait WorldQuery {
    type Fetch: for<'a> Fetch<'a, State = Self::State>;
    type State: FetchState;
}

pub trait Fetch<'w>: Sized {
    type Item;
    type State: FetchState;

    /// Creates a new instance of this fetch.
    /// # Safety
    /// `state` must have been initialized (via [FetchState::init]) using the same `world` passed in to this function.
    unsafe fn init(world: &World, state: &Self::State) -> Self;

    /// Returns true if (and only if) every table of every archetype matched by this Fetch contains all of the matched components.
    /// This is used to select a more efficient "table iterator" for "dense" queries.
    /// If this returns true, [Fetch::set_table] and [Fetch::table_fetch] will be called for iterators
    /// If this returns false, [Fetch::set_archetype] and [Fetch::archetype_fetch] will be called for iterators
    fn is_dense(&self) -> bool;

    /// Adjusts internal state to account for the next [Archetype]. This will always be called on archetypes that match this [Fetch]
    /// # Safety
    /// `archetype` and `tables` must be from the [World] [Fetch::init] was called on. `state` must be the [Self::State] this was initialized with.
    unsafe fn set_archetype(&mut self, state: &Self::State, archetype: &Archetype, tables: &Tables);

    /// Adjusts internal state to account for the next [Table]. This will always be called on tables that match this [Fetch]
    /// # Safety
    /// `table` must be from the [World] [Fetch::init] was called on. `state` must be the [Self::State] this was initialized with.
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table);

    /// Fetch [Self::Item] for the given `archetype_index` in the current [Archetype]. This must always be called after [Fetch::set_archetype] with an `archetype_index`
    /// in the range of the current [Archetype]
    /// # Safety
    /// Must always be called _after_ [Fetch::set_archetype]. `archetype_index` must be in the range of the current archetype
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item;

    /// Fetch [Self::Item] for the given `table_row` in the current [Table]. This must always be called after [Fetch::set_table] with a `table_row`
    /// in the range of the current [Table]
    /// # Safety
    /// Must always be called _after_ [Fetch::set_table]. `table_row` must be in the range of the current table
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item;
}

/// State used to construct a Fetch. This will be cached inside QueryState, so it is best to move as much data /
/// computation here as possible to reduce the cost of constructing Fetch.
/// SAFETY:
/// Implementor must ensure that [FetchState::update_component_access] and [FetchState::update_archetype_component_access] exactly
/// reflects the results of [FetchState::matches_archetype], [FetchState::matches_table], [Fetch::archetype_fetch], and [Fetch::table_fetch]
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

pub struct EntityFetch {
    entities: *const Entity,
}

/// SAFE: access is read only
unsafe impl ReadOnlyFetch for EntityFetch {}

pub struct EntityState;

// SAFE: no component or archetype access
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

impl<'w> Fetch<'w> for EntityFetch {
    type Item = Entity;
    type State = EntityState;

    #[inline]
    fn is_dense(&self) -> bool {
        true
    }

    unsafe fn init(_world: &World, _state: &Self::State) -> Self {
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

pub struct ReadState<T> {
    component_id: ComponentId,
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFE: component access and archetype component access are properly updated to reflect that T is read
unsafe impl<T: Component> FetchState for ReadState<T> {
    fn init(world: &mut World) -> Self {
        let component_info = world.components.get_or_insert_info::<T>();
        ReadState {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
            marker: PhantomData,
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
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

pub struct ReadFetch<T> {
    storage_type: StorageType,
    table_components: NonNull<T>,
    entity_table_rows: *const usize,
    entities: *const Entity,
    sparse_set: *const ComponentSparseSet,
}

/// SAFE: access is read only
unsafe impl<T> ReadOnlyFetch for ReadFetch<T> {}

impl<'w, T: Component> Fetch<'w> for ReadFetch<T> {
    type Item = &'w T;
    type State = ReadState<T>;

    #[inline]
    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn init(world: &World, state: &Self::State) -> Self {
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
        match state.storage_type {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                // SAFE: archetype tables always exist
                let table = tables.get_unchecked(archetype.table_id());
                let column = table.get_column(state.component_id).unwrap();
                self.table_components = column.get_ptr().cast::<T>();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        self.table_components = table
            .get_column(state.component_id)
            .unwrap()
            .get_ptr()
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

pub struct WriteFetch<T> {
    storage_type: StorageType,
    table_components: NonNull<T>,
    table_flags: *mut ComponentFlags,
    entities: *const Entity,
    entity_table_rows: *const usize,
    sparse_set: *const ComponentSparseSet,
}

pub struct WriteState<T> {
    component_id: ComponentId,
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFE: component access and archetype component access are properly updated to reflect that T is written
unsafe impl<T: Component> FetchState for WriteState<T> {
    fn init(world: &mut World) -> Self {
        let component_info = world.components.get_or_insert_info::<T>();
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

impl<'w, T: Component> Fetch<'w> for WriteFetch<T> {
    type Item = Mut<'w, T>;
    type State = WriteState<T>;

    #[inline]
    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn init(world: &World, state: &Self::State) -> Self {
        let mut value = Self {
            storage_type: state.storage_type,
            table_components: NonNull::dangling(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
            table_flags: ptr::null_mut::<ComponentFlags>(),
        };
        if state.storage_type == StorageType::SparseSet {
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
        match state.storage_type {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                // SAFE: archetype tables always exist
                let table = tables.get_unchecked(archetype.table_id());
                let column = table.get_column(state.component_id).unwrap();
                self.table_components = column.get_ptr().cast::<T>();
                self.table_flags = column.get_flags_mut_ptr();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        let column = table.get_column(state.component_id).unwrap();
        self.table_components = column.get_ptr().cast::<T>();
        self.table_flags = column.get_flags_mut_ptr();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
            StorageType::Table => {
                let table_row = *self.entity_table_rows.add(archetype_index);
                Mut {
                    value: &mut *self.table_components.as_ptr().add(table_row),
                    flags: &mut *self.table_flags.add(table_row),
                }
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                let (component, flags) = (*self.sparse_set).get_with_flags(entity).unwrap();
                Mut {
                    value: &mut *component.cast::<T>(),
                    flags: &mut *flags,
                }
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        Mut {
            value: &mut *self.table_components.as_ptr().add(table_row),
            flags: &mut *self.table_flags.add(table_row),
        }
    }
}

impl<T: WorldQuery> WorldQuery for Option<T> {
    type Fetch = OptionFetch<T::Fetch>;
    type State = OptionState<T::State>;
}

pub struct OptionFetch<T> {
    fetch: T,
    matches: bool,
}

/// SAFE: OptionFetch is read only because T is read only
unsafe impl<T: ReadOnlyFetch> ReadOnlyFetch for OptionFetch<T> {}

pub struct OptionState<T: FetchState> {
    state: T,
}

// SAFE: component access and archetype component access are properly updated according to the internal Fetch
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

impl<'w, T: Fetch<'w>> Fetch<'w> for OptionFetch<T> {
    type Item = Option<T::Item>;
    type State = OptionState<T::State>;

    #[inline]
    fn is_dense(&self) -> bool {
        self.fetch.is_dense()
    }

    unsafe fn init(world: &World, state: &Self::State) -> Self {
        Self {
            fetch: T::init(world, &state.state),
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

/// Flags on component `T` that happened since the start of the frame.
#[derive(Clone)]
pub struct Flags<T: Component> {
    flags: ComponentFlags,
    marker: PhantomData<T>,
}
impl<T: Component> std::fmt::Debug for Flags<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Flags")
            .field("added", &self.added())
            .field("mutated", &self.mutated())
            .finish()
    }
}

impl<T: Component> Flags<T> {
    /// Has this component been added since the start of the frame.
    pub fn added(&self) -> bool {
        self.flags.contains(ComponentFlags::ADDED)
    }

    /// Has this component been mutated since the start of the frame.
    pub fn mutated(&self) -> bool {
        self.flags.contains(ComponentFlags::MUTATED)
    }

    /// Has this component been either mutated or added since the start of the frame.
    pub fn changed(&self) -> bool {
        self.flags
            .intersects(ComponentFlags::ADDED | ComponentFlags::MUTATED)
    }
}

impl<T: Component> WorldQuery for Flags<T> {
    type Fetch = FlagsFetch<T>;
    type State = FlagsState<T>;
}

pub struct FlagsState<T> {
    component_id: ComponentId,
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFE: component access and archetype component access are properly updated to reflect that T is read
unsafe impl<T: Component> FetchState for FlagsState<T> {
    fn init(world: &mut World) -> Self {
        let component_info = world.components.get_or_insert_info::<T>();
        Self {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
            marker: PhantomData,
        }
    }

    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
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

pub struct FlagsFetch<T> {
    storage_type: StorageType,
    table_flags: *const ComponentFlags,
    entity_table_rows: *const usize,
    entities: *const Entity,
    sparse_set: *const ComponentSparseSet,
    marker: PhantomData<T>,
}

/// SAFE: access is read only  
unsafe impl<T> ReadOnlyFetch for FlagsFetch<T> {}

impl<'w, T: Component> Fetch<'w> for FlagsFetch<T> {
    type Item = Flags<T>;
    type State = FlagsState<T>;

    #[inline]
    fn is_dense(&self) -> bool {
        match self.storage_type {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    }

    unsafe fn init(world: &World, state: &Self::State) -> Self {
        let mut value = Self {
            storage_type: state.storage_type,
            table_flags: ptr::null::<ComponentFlags>(),
            entities: ptr::null::<Entity>(),
            entity_table_rows: ptr::null::<usize>(),
            sparse_set: ptr::null::<ComponentSparseSet>(),
            marker: PhantomData,
        };
        if state.storage_type == StorageType::SparseSet {
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
        match state.storage_type {
            StorageType::Table => {
                self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                // SAFE: archetype tables always exist
                let table = tables.get_unchecked(archetype.table_id());
                let column = table.get_column(state.component_id).unwrap();
                self.table_flags = column.get_flags_mut_ptr().cast::<ComponentFlags>();
            }
            StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
        }
    }

    #[inline]
    unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
        self.table_flags = table
            .get_column(state.component_id)
            .unwrap()
            .get_flags_mut_ptr()
            .cast::<ComponentFlags>();
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> Self::Item {
        match self.storage_type {
            StorageType::Table => {
                let table_row = *self.entity_table_rows.add(archetype_index);
                Flags {
                    flags: *self.table_flags.add(table_row),
                    marker: PhantomData,
                }
            }
            StorageType::SparseSet => {
                let entity = *self.entities.add(archetype_index);
                Flags {
                    flags: *(*self.sparse_set).get_flags(entity).unwrap(),
                    marker: PhantomData,
                }
            }
        }
    }

    #[inline]
    unsafe fn table_fetch(&mut self, table_row: usize) -> Self::Item {
        Flags {
            flags: *self.table_flags.add(table_row),
            marker: PhantomData,
        }
    }
}

macro_rules! impl_tuple_fetch {
    ($(($name: ident, $state: ident)),*) => {
        #[allow(non_snake_case)]
        impl<'a, $($name: Fetch<'a>),*> Fetch<'a> for ($($name,)*) {
            type Item = ($($name::Item,)*);
            type State = ($($name::State,)*);

            unsafe fn init(_world: &World, state: &Self::State) -> Self {
                let ($($name,)*) = state;
                ($($name::init(_world, $name),)*)
            }


            #[inline]
            fn is_dense(&self) -> bool {
                let ($($name,)*) = self;
                true $(&& $name.is_dense())*
            }

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
            unsafe fn table_fetch(&mut self, _table_row: usize) -> Self::Item {
                let ($($name,)*) = self;
                ($($name.table_fetch(_table_row),)*)
            }

            #[inline]
            unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
                let ($($name,)*) = self;
                ($($name.archetype_fetch(_archetype_index),)*)
            }
        }

        // SAFE: update_component_access and update_archetype_component_access are called for each item in the tuple
        #[allow(non_snake_case)]
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

        /// SAFE: each item in the tuple is read only
        unsafe impl<$($name: ReadOnlyFetch),*> ReadOnlyFetch for ($($name,)*) {}

    };
}

all_tuples!(impl_tuple_fetch, 0, 15, F, S);
