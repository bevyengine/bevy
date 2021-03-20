use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    bundle::Bundle,
    component::{Component, ComponentId, ComponentTicks, StorageType},
    entity::Entity,
    query::{Access, Fetch, FetchState, FilteredAccess, WorldQuery},
    storage::{ComponentSparseSet, Table, Tables},
    world::World,
};
use bevy_ecs_macros::all_tuples;
use std::{marker::PhantomData, ptr};

// TODO: uncomment this and use as shorthand (remove where F::Fetch: FilterFetch everywhere) when
// this bug is fixed in Rust 1.51: https://github.com/rust-lang/rust/pull/81671
// pub trait QueryFilter: WorldQuery
// where
//     Self::Fetch: FilterFetch,
// {
// }

// impl<T: WorldQuery> QueryFilter for T where T::Fetch: FilterFetch {
// }

/// Fetch methods used by query filters. This trait exists to allow "short circuit" behaviors for
/// relevant query filter fetches.
pub trait FilterFetch: for<'a> Fetch<'a> {
    /// # Safety
    /// Must always be called _after_ [Fetch::set_archetype]. `archetype_index` must be in the range
    /// of the current archetype
    unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool;

    /// # Safety
    /// Must always be called _after_ [Fetch::set_table]. `table_row` must be in the range of the
    /// current table
    unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool;
}

impl<T> FilterFetch for T
where
    T: for<'a> Fetch<'a, Item = bool>,
{
    #[inline]
    unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool {
        self.archetype_fetch(archetype_index)
    }

    #[inline]
    unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool {
        self.table_fetch(table_row)
    }
}

/// Filter that selects entities with a component `T`
pub struct With<T>(PhantomData<T>);

impl<T: Component> WorldQuery for With<T> {
    type Fetch = WithFetch<T>;
    type State = WithState<T>;
}

pub struct WithFetch<T> {
    storage_type: StorageType,
    marker: PhantomData<T>,
}
pub struct WithState<T> {
    component_id: ComponentId,
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFE: no component access or archetype component access
unsafe impl<T: Component> FetchState for WithState<T> {
    fn init(world: &mut World) -> Self {
        let component_info = world.components.get_or_insert_info::<T>();
        Self {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
            marker: PhantomData,
        }
    }

    #[inline]
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        access.add_with(self.component_id);
    }

    #[inline]
    fn update_archetype_component_access(
        &self,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        archetype.contains(self.component_id)
    }

    fn matches_table(&self, table: &Table) -> bool {
        table.has_column(self.component_id)
    }
}

impl<'a, T: Component> Fetch<'a> for WithFetch<T> {
    type Item = bool;
    type State = WithState<T>;

    unsafe fn init(
        _world: &World,
        state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        Self {
            storage_type: state.storage_type,
            marker: PhantomData,
        }
    }

    #[inline]
    fn is_dense(&self) -> bool {
        self.storage_type == StorageType::Table
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, _table: &Table) {}

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        _archetype: &Archetype,
        _tables: &Tables,
    ) {
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> Self::Item {
        true
    }

    #[inline]
    unsafe fn table_fetch(&mut self, _table_row: usize) -> bool {
        true
    }
}

/// Filter that selects entities without a component `T`
pub struct Without<T>(PhantomData<T>);

impl<T: Component> WorldQuery for Without<T> {
    type Fetch = WithoutFetch<T>;
    type State = WithoutState<T>;
}

pub struct WithoutFetch<T> {
    storage_type: StorageType,
    marker: PhantomData<T>,
}

pub struct WithoutState<T> {
    component_id: ComponentId,
    storage_type: StorageType,
    marker: PhantomData<T>,
}

// SAFE: no component access or archetype component access
unsafe impl<T: Component> FetchState for WithoutState<T> {
    fn init(world: &mut World) -> Self {
        let component_info = world.components.get_or_insert_info::<T>();
        Self {
            component_id: component_info.id(),
            storage_type: component_info.storage_type(),
            marker: PhantomData,
        }
    }

    #[inline]
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        access.add_without(self.component_id);
    }

    #[inline]
    fn update_archetype_component_access(
        &self,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        !archetype.contains(self.component_id)
    }

    fn matches_table(&self, table: &Table) -> bool {
        !table.has_column(self.component_id)
    }
}

impl<'a, T: Component> Fetch<'a> for WithoutFetch<T> {
    type Item = bool;
    type State = WithoutState<T>;

    unsafe fn init(
        _world: &World,
        state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        Self {
            storage_type: state.storage_type,
            marker: PhantomData,
        }
    }

    #[inline]
    fn is_dense(&self) -> bool {
        self.storage_type == StorageType::Table
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, _table: &Table) {}

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        _archetype: &Archetype,
        _tables: &Tables,
    ) {
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> bool {
        true
    }

    #[inline]
    unsafe fn table_fetch(&mut self, _table_row: usize) -> bool {
        true
    }
}

pub struct WithBundle<T: Bundle>(PhantomData<T>);

pub struct WithBundleFetch<T: Bundle> {
    is_dense: bool,
    marker: PhantomData<T>,
}

pub struct WithBundleState<T: Bundle> {
    component_ids: Vec<ComponentId>,
    is_dense: bool,
    marker: PhantomData<T>,
}

// SAFE: no component access or archetype component access
unsafe impl<T: Bundle> FetchState for WithBundleState<T> {
    fn init(world: &mut World) -> Self {
        let bundle_info = world.bundles.init_info::<T>(&mut world.components);
        let components = &world.components;
        Self {
            component_ids: bundle_info.component_ids.clone(),
            is_dense: !bundle_info.component_ids.iter().any(|id| unsafe {
                components.get_info_unchecked(*id).storage_type() != StorageType::Table
            }),
            marker: PhantomData,
        }
    }

    #[inline]
    fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
        for component_id in self.component_ids.iter().cloned() {
            access.add_with(component_id);
        }
    }

    #[inline]
    fn update_archetype_component_access(
        &self,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }

    fn matches_archetype(&self, archetype: &Archetype) -> bool {
        self.component_ids.iter().all(|id| archetype.contains(*id))
    }

    fn matches_table(&self, table: &Table) -> bool {
        self.component_ids.iter().all(|id| table.has_column(*id))
    }
}

impl<'a, T: Bundle> Fetch<'a> for WithBundleFetch<T> {
    type Item = bool;
    type State = WithBundleState<T>;

    unsafe fn init(
        _world: &World,
        state: &Self::State,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        Self {
            is_dense: state.is_dense,
            marker: PhantomData,
        }
    }

    #[inline]
    fn is_dense(&self) -> bool {
        self.is_dense
    }

    #[inline]
    unsafe fn set_table(&mut self, _state: &Self::State, _table: &Table) {}

    #[inline]
    unsafe fn set_archetype(
        &mut self,
        _state: &Self::State,
        _archetype: &Archetype,
        _tables: &Tables,
    ) {
    }

    #[inline]
    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) -> bool {
        true
    }

    #[inline]
    unsafe fn table_fetch(&mut self, _table_row: usize) -> bool {
        true
    }
}

pub struct Or<T>(pub T);
pub struct OrFetch<T: FilterFetch> {
    fetch: T,
    matches: bool,
}

macro_rules! impl_query_filter_tuple {
    ($(($filter: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'a, $($filter: FilterFetch),*> FilterFetch for ($($filter,)*) {
            #[inline]
            unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool {
                let ($($filter,)*) = self;
                true $(&& $filter.table_filter_fetch(table_row))*
            }

            #[inline]
            unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool {
                let ($($filter,)*) = self;
                true $(&& $filter.archetype_filter_fetch(archetype_index))*
            }
        }

        impl<$($filter: WorldQuery),*> WorldQuery for Or<($($filter,)*)>
            where $($filter::Fetch: FilterFetch),*
        {
            type Fetch = Or<($(OrFetch<$filter::Fetch>,)*)>;
            type State = Or<($($filter::State,)*)>;
        }


        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'a, $($filter: FilterFetch),*> Fetch<'a> for Or<($(OrFetch<$filter>,)*)> {
            type State = Or<($(<$filter as Fetch<'a>>::State,)*)>;
            type Item = bool;

            unsafe fn init(world: &World, state: &Self::State, last_change_tick: u32, change_tick: u32) -> Self {
                let ($($filter,)*) = &state.0;
                Or(($(OrFetch {
                    fetch: $filter::init(world, $filter, last_change_tick, change_tick),
                    matches: false,
                },)*))
            }

            #[inline]
            fn is_dense(&self) -> bool {
                let ($($filter,)*) = &self.0;
                true $(&& $filter.fetch.is_dense())*
            }

            #[inline]
            unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
                let ($($filter,)*) = &mut self.0;
                let ($($state,)*) = &state.0;
                $(
                    $filter.matches = $state.matches_table(table);
                    if $filter.matches {
                        $filter.fetch.set_table($state, table);
                    }
                )*
            }

            #[inline]
            unsafe fn set_archetype(&mut self, state: &Self::State, archetype: &Archetype, tables: &Tables) {
                let ($($filter,)*) = &mut self.0;
                let ($($state,)*) = &state.0;
                $(
                    $filter.matches = $state.matches_archetype(archetype);
                    if $filter.matches {
                        $filter.fetch.set_archetype($state, archetype, tables);
                    }
                )*
            }

            #[inline]
            unsafe fn table_fetch(&mut self, table_row: usize) -> bool {
                let ($($filter,)*) = &mut self.0;
                false $(|| ($filter.matches && $filter.fetch.table_filter_fetch(table_row)))*
            }

            #[inline]
            unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> bool {
                let ($($filter,)*) = &mut self.0;
                false $(|| ($filter.matches && $filter.fetch.archetype_filter_fetch(archetype_index)))*
            }
        }

        // SAFE: update_component_access and update_archetype_component_access are called for each item in the tuple
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        unsafe impl<$($filter: FetchState),*> FetchState for Or<($($filter,)*)> {
            fn init(world: &mut World) -> Self {
                Or(($($filter::init(world),)*))
            }

            fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
                let ($($filter,)*) = &self.0;
                $($filter.update_component_access(access);)*
            }

            fn update_archetype_component_access(&self, archetype: &Archetype, access: &mut Access<ArchetypeComponentId>) {
                let ($($filter,)*) = &self.0;
                $($filter.update_archetype_component_access(archetype, access);)*
            }

            fn matches_archetype(&self, archetype: &Archetype) -> bool {
                let ($($filter,)*) = &self.0;
                false $(|| $filter.matches_archetype(archetype))*
            }

            fn matches_table(&self, table: &Table) -> bool {
                let ($($filter,)*) = &self.0;
                false $(|| $filter.matches_table(table))*
            }
        }
    };
}

all_tuples!(impl_query_filter_tuple, 0, 15, F, S);

macro_rules! impl_tick_filter {
    (
        $(#[$meta:meta])*
        $name: ident, $state_name: ident, $fetch_name: ident, $is_detected: expr) => {
        $(#[$meta])*
        pub struct $name<T>(PhantomData<T>);

        pub struct $fetch_name<T> {
            storage_type: StorageType,
            table_ticks: *mut ComponentTicks,
            entity_table_rows: *const usize,
            marker: PhantomData<T>,
            entities: *const Entity,
            sparse_set: *const ComponentSparseSet,
            last_change_tick: u32,
            change_tick: u32,
        }

        pub struct $state_name<T> {
            component_id: ComponentId,
            storage_type: StorageType,
            marker: PhantomData<T>,
        }

        impl<T: Component> WorldQuery for $name<T> {
            type Fetch = $fetch_name<T>;
            type State = $state_name<T>;
        }


        // SAFE: this reads the T component. archetype component access and component access are updated to reflect that
        unsafe impl<T: Component> FetchState for $state_name<T> {
            fn init(world: &mut World) -> Self {
                let component_info = world.components.get_or_insert_info::<T>();
                Self {
                    component_id: component_info.id(),
                    storage_type: component_info.storage_type(),
                    marker: PhantomData,
                }
            }

            #[inline]
            fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
                access.add_read(self.component_id);
            }

            #[inline]
            fn update_archetype_component_access(
                &self,
                archetype: &Archetype,
                access: &mut Access<ArchetypeComponentId>,
            ) {
                if let Some(archetype_component_id) = archetype.get_archetype_component_id(self.component_id) {
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

        impl<'a, T: Component> Fetch<'a> for $fetch_name<T> {
            type State = $state_name<T>;
            type Item = bool;

            unsafe fn init(world: &World, state: &Self::State, last_change_tick: u32, change_tick: u32) -> Self {
                let mut value = Self {
                    storage_type: state.storage_type,
                    table_ticks: ptr::null_mut::<ComponentTicks>(),
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
                        .get(state.component_id).unwrap();
                }
                value
            }

            #[inline]
            fn is_dense(&self) -> bool {
                self.storage_type == StorageType::Table
            }

            unsafe fn set_table(&mut self, state: &Self::State, table: &Table) {
                self.table_ticks = table
                    .get_column(state.component_id).unwrap()
                    .get_ticks_mut_ptr();
            }

            unsafe fn set_archetype(&mut self, state: &Self::State, archetype: &Archetype, tables: &Tables) {
                match state.storage_type {
                    StorageType::Table => {
                        self.entity_table_rows = archetype.entity_table_rows().as_ptr();
                        let table = &tables[archetype.table_id()];
                        self.table_ticks = table
                            .get_column(state.component_id).unwrap()
                            .get_ticks_mut_ptr();
                    }
                    StorageType::SparseSet => self.entities = archetype.entities().as_ptr(),
                }
            }

            unsafe fn table_fetch(&mut self, table_row: usize) -> bool {
                $is_detected(&*self.table_ticks.add(table_row), self.last_change_tick, self.change_tick)
            }

            unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> bool {
                match self.storage_type {
                    StorageType::Table => {
                        let table_row = *self.entity_table_rows.add(archetype_index);
                        $is_detected(&*self.table_ticks.add(table_row), self.last_change_tick, self.change_tick)
                    }
                    StorageType::SparseSet => {
                        let entity = *self.entities.add(archetype_index);
                        let ticks = (*(*self.sparse_set).get_ticks(entity).unwrap());
                        $is_detected(&ticks, self.last_change_tick, self.change_tick)
                    }
                }
            }
        }
    };
}

impl_tick_filter!(
    /// Filter that retrieves components of type `T` that have been added since the last execution
    /// of this system
    ///
    /// This filter is useful to do one-time post-processing on components.
    ///
    /// Because the ordering of systems can change and this filter is only effective on changes
    /// before the query executes you need to use explicit dependency ordering or ordered stages to
    /// avoid frame delays.
    ///
    ///
    /// Example:
    /// ```
    /// # use bevy_ecs::system::Query;
    /// # use bevy_ecs::query::Added;
    /// #
    /// # #[derive(Debug)]
    /// # struct Name {};
    /// # struct Transform {};
    /// #
    /// fn print_add_name_component(query: Query<&Name, Added<Name>>) {
    ///     for name in query.iter() {
    ///         println!("Named entity created: {:?}", name)
    ///     }
    /// }
    /// ```
    Added,
    AddedState,
    AddedFetch,
    ComponentTicks::is_added
);

impl_tick_filter!(
    /// Filter that retrieves components of type `T` that have been changed since the last
    /// execution of this system
    ///
    /// This filter is useful for synchronizing components, and as a performance optimization as it
    /// means that the query contains fewer items for a system to iterate over.
    ///
    /// Because the ordering of systems can change and this filter is only effective on changes
    /// before the query executes you need to use explicit dependency ordering or ordered
    /// stages to avoid frame delays.
    ///
    /// Example:
    /// ```
    /// # use bevy_ecs::system::Query;
    /// # use bevy_ecs::query::Changed;
    /// #
    /// # #[derive(Debug)]
    /// # struct Name {};
    /// # struct Transform {};
    /// #
    /// fn print_moving_objects_system(query: Query<&Name, Changed<Transform>>) {
    ///     for name in query.iter() {
    ///         println!("Entity Moved: {:?}", name);
    ///     }
    /// }
    /// ```
    Changed,
    ChangedState,
    ChangedFetch,
    ComponentTicks::is_changed
);
