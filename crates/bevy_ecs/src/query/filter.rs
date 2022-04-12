use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{Component, ComponentId, ComponentStorage, ComponentTicks, StorageType},
    entity::Entity,
    query::{
        debug_checked_unreachable, Access, Fetch, FetchInit, FetchState, FilteredAccess, WorldQuery,
    },
    storage::{ComponentSparseSet, Table, Tables},
    world::World,
};
use bevy_ecs_macros::all_tuples;
use std::{cell::UnsafeCell, marker::PhantomData};

use super::ReadOnlyFetch;

/// Extension trait for [`Fetch`] containing methods used by query filters.
/// This trait exists to allow "short circuit" behaviors for relevant query filter fetches.
pub trait FilterFetch<'w, 's>: Fetch<'w, 's> {
    /// # Safety
    ///
    /// Must always be called _after_ [`Fetch::set_archetype`]. `archetype_index` must be in the range
    /// of the current archetype.
    unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool;

    /// # Safety
    ///
    /// Must always be called _after_ [`Fetch::set_table`]. `table_row` must be in the range of the
    /// current table.
    unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool;
}

impl<'w, 's, T> FilterFetch<'w, 's> for T
where
    T: Fetch<'w, 's, Item = bool>,
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

/// Filter that selects entities with a component `T`.
///
/// This can be used in a [`Query`](crate::system::Query) if entities are required to have the
/// component `T` but you don't actually care about components value.
///
/// This is the negation of [`Without`].
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::query::With;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component)]
/// # struct IsBeautiful;
/// # #[derive(Component)]
/// # struct Name { name: &'static str };
/// #
/// fn compliment_entity_system(query: Query<&Name, With<IsBeautiful>>) {
///     for name in query.iter() {
///         println!("{} is looking lovely today!", name.name);
///     }
/// }
/// # bevy_ecs::system::assert_is_system(compliment_entity_system);
/// ```
pub struct With<T>(PhantomData<T>);

impl<T: Component> WorldQuery for With<T> {
    type State = WithState<T>;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: super::QueryItem<'wlong, 'slong, Self>,
    ) -> super::QueryItem<'wshort, 'sshort, Self> {
        item
    }
}

/// The [`Fetch`] of [`With`].
#[doc(hidden)]
pub struct WithFetch<T> {
    marker: PhantomData<T>,
}

/// The [`FetchState`] of [`With`].
#[doc(hidden)]
pub struct WithState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> FetchState for WithState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        Self {
            component_id,
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

impl<T: Component> FetchInit<'_, '_> for WithState<T> {
    type Fetch = WithFetch<T>;
    type Item = bool;
    type ReadOnlyFetch = WithFetch<T>;
    type ReadOnlyItem = bool;
}

impl<'w, 's, T: Component> Fetch<'w, 's> for WithFetch<T> {
    type Item = bool;
    type State = WithState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        _world: &World,
        _state: &WithState<T>,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        Self {
            marker: PhantomData,
        }
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

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> ReadOnlyFetch<'_, '_> for WithFetch<T> {}

impl<T> Clone for WithFetch<T> {
    fn clone(&self) -> Self {
        Self {
            marker: self.marker,
        }
    }
}

impl<T> Copy for WithFetch<T> {}

/// Filter that selects entities without a component `T`.
///
/// This is the negation of [`With`].
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::query::Without;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component)]
/// # struct Permit;
/// # #[derive(Component)]
/// # struct Name { name: &'static str };
/// #
/// fn no_permit_system(query: Query<&Name, Without<Permit>>) {
///     for name in query.iter() {
///         println!("{} has no permit!", name.name);
///     }
/// }
/// # bevy_ecs::system::assert_is_system(no_permit_system);
/// ```
pub struct Without<T>(PhantomData<T>);

impl<T: Component> WorldQuery for Without<T> {
    type State = WithoutState<T>;

    fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
        item: super::QueryItem<'wlong, 'slong, Self>,
    ) -> super::QueryItem<'wshort, 'sshort, Self> {
        item
    }
}

/// The [`Fetch`] of [`Without`].
#[doc(hidden)]
pub struct WithoutFetch<T> {
    marker: PhantomData<T>,
}

/// The [`FetchState`] of [`Without`].
#[doc(hidden)]
pub struct WithoutState<T> {
    component_id: ComponentId,
    marker: PhantomData<T>,
}

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> FetchState for WithoutState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        Self {
            component_id,
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

impl<T: Component> FetchInit<'_, '_> for WithoutState<T> {
    type Fetch = WithoutFetch<T>;
    type Item = bool;

    type ReadOnlyFetch = WithoutFetch<T>;
    type ReadOnlyItem = bool;
}

impl<'w, 's, T: Component> Fetch<'w, 's> for WithoutFetch<T> {
    type Item = bool;
    type State = WithoutState<T>;

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    unsafe fn init(
        _world: &World,
        _state: &WithoutState<T>,
        _last_change_tick: u32,
        _change_tick: u32,
    ) -> Self {
        WithoutFetch {
            marker: PhantomData,
        }
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

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> ReadOnlyFetch<'_, '_> for WithoutFetch<T> {}

impl<T> Clone for WithoutFetch<T> {
    fn clone(&self) -> Self {
        Self {
            marker: self.marker,
        }
    }
}

impl<T> Copy for WithoutFetch<T> {}

/// A filter that tests if any of the given filters apply.
///
/// This is useful for example if a system with multiple components in a query only wants to run
/// when one or more of the components have changed.
///
/// The `And` equivalent to this filter is a [`prim@tuple`] testing that all the contained filters
/// apply instead.
///
/// # Examples
///
/// ```
/// # use bevy_ecs::component::Component;
/// # use bevy_ecs::entity::Entity;
/// # use bevy_ecs::query::Changed;
/// # use bevy_ecs::query::Or;
/// # use bevy_ecs::system::IntoSystem;
/// # use bevy_ecs::system::Query;
/// #
/// # #[derive(Component, Debug)]
/// # struct Color {};
/// # #[derive(Component)]
/// # struct Style {};
/// #
/// fn print_cool_entity_system(query: Query<Entity, Or<(Changed<Color>, Changed<Style>)>>) {
///     for entity in query.iter() {
///         println!("Entity {:?} got a new style or color", entity);
///     }
/// }
/// # bevy_ecs::system::assert_is_system(print_cool_entity_system);
/// ```
#[derive(Clone, Copy)]
pub struct Or<T>(pub T);

/// The [`Fetch`] of [`Or`].
#[derive(Clone)]
pub struct OrFetch<'w, 's, T: FilterFetch<'w, 's>> {
    fetch: T,
    matches: bool,
    _marker: PhantomData<(&'w (), &'s ())>,
}

macro_rules! impl_query_filter_tuple {
    ($(($filter: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, 's, $($filter: FilterFetch<'w, 's>),*> FilterFetch<'w, 's> for ($($filter,)*) {
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

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($filter: WorldQuery),*> WorldQuery for Or<($($filter,)*)>
            where $(for<'w, 's> <$filter::State as FetchInit<'w, 's>>::Fetch: FilterFetch<'w, 's>),*
        {
            type State = Or<($($filter::State,)*)>;

            fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
                item: super::QueryItem<'wlong, 'slong, Self>,
            ) -> super::QueryItem<'wshort, 'sshort, Self> {
                item
            }
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, 's, $($filter: FetchInit<'w, 's>),*> FetchInit<'w, 's> for Or<($($filter,)*)>
            where $($filter::Fetch: FilterFetch<'w, 's>),*
        {
            type Fetch = Or<($(OrFetch<'w, 's, $filter::Fetch>,)*)>;
            type Item = bool;
            type ReadOnlyFetch = Or<($(OrFetch<'w, 's, $filter::Fetch>,)*)>;
            type ReadOnlyItem = bool;
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, 's, $($filter: FilterFetch<'w, 's>),*> Fetch<'w, 's> for Or<($(OrFetch<'w, 's, $filter>,)*)> {
            type State = Or<($(<$filter as Fetch<'w, 's>>::State,)*)>;
            type Item = bool;

            const IS_DENSE: bool = true $(&& $filter::IS_DENSE)*;

            unsafe fn init(world: &'w World, state: &'s Or<($(<$filter as Fetch<'w, 's>>::State,)*)>, last_change_tick: u32, change_tick: u32) -> Self {
                let ($($filter,)*) = &state.0;
                Or(($(OrFetch {
                    fetch: <$filter as Fetch<'w, 's>>::init(world, $filter, last_change_tick, change_tick),
                    matches: false,
                    _marker: PhantomData,
                },)*))
            }

            #[inline]
            unsafe fn set_table(&mut self, state: &'s Self::State, table: &'w Table) {
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
            unsafe fn set_archetype(&mut self, state: &'s Self::State, archetype: &'w Archetype, tables: &'w Tables) {
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

        // SAFETY: update_component_access and update_archetype_component_access are called for each item in the tuple
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

        // SAFE: filters are read only
        unsafe impl<'w, 's, $($filter: FilterFetch<'w, 's>),*> ReadOnlyFetch<'w, 's> for Or<($(OrFetch<'w, 's, $filter>,)*)> {}
    };
}

all_tuples!(impl_query_filter_tuple, 0, 15, F, S);

macro_rules! impl_tick_filter {
    (
        $(#[$meta:meta])*
        $name: ident,
        $(#[$state_meta:meta])*
        $state_name: ident,
        $(#[$fetch_meta:meta])*
        $fetch_name: ident,
        $is_detected: expr
    ) => {
        $(#[$meta])*
        pub struct $name<T>(PhantomData<T>);

        #[doc(hidden)]
        $(#[$fetch_meta])*
        pub struct $fetch_name<'w, T> {
            table_ticks: Option<&'w [UnsafeCell<ComponentTicks>]>,
            entity_table_rows: Option<&'w [usize]>,
            marker: PhantomData<T>,
            entities: Option<&'w [Entity]>,
            sparse_set: Option<&'w ComponentSparseSet>,
            last_change_tick: u32,
            change_tick: u32,
        }

        impl<T> Clone for $fetch_name<'_, T> {
            fn clone(&self) -> Self {
                Self {
                    table_ticks: self.table_ticks.clone(),
                    entity_table_rows: self.entity_table_rows.clone(),
                    marker: self.marker.clone(),
                    entities: self.entities.clone(),
                    sparse_set: self.sparse_set.clone(),
                    last_change_tick: self.last_change_tick.clone(),
                    change_tick: self.change_tick.clone(),
                }
            }
        }

        #[doc(hidden)]
        $(#[$state_meta])*
        pub struct $state_name<T> {
            component_id: ComponentId,
            marker: PhantomData<T>,
        }

        impl<T: Component> WorldQuery for $name<T> {
            type State = $state_name<T>;

            fn shrink<'wlong: 'wshort, 'slong: 'sshort, 'wshort, 'sshort>(
                item: super::QueryItem<'wlong, 'slong, Self>,
            ) -> super::QueryItem<'wshort, 'sshort, Self> {
                item
            }
        }

        // SAFETY: this reads the T component. archetype component access and component access are updated to reflect that
        unsafe impl<T: Component> FetchState for $state_name<T> {
            fn init(world: &mut World) -> Self {
                Self {
                    component_id: world.init_component::<T>(),
                    marker: PhantomData,
                }
            }

            #[inline]
            fn update_component_access(&self, access: &mut FilteredAccess<ComponentId>) {
                if access.access().has_write(self.component_id) {
                    panic!("$state_name<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                        std::any::type_name::<T>());
                }
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

        impl<'w, T: Component> FetchInit<'w, '_> for $state_name<T> {
            type Fetch = $fetch_name<'w, T>;
            type Item = bool;
            type ReadOnlyFetch = $fetch_name<'w, T>;
            type ReadOnlyItem = bool;
        }

        impl<'w, 's, T: Component> Fetch<'w, 's> for $fetch_name<'w, T> {
            type State = $state_name<T>;
            type Item = bool;

            const IS_DENSE: bool = {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => true,
                    StorageType::SparseSet => false,
                }
            };

            unsafe fn init(world: &'w World, state: &'s $state_name<T>, last_change_tick: u32, change_tick: u32) -> Self {
                Self {
                    table_ticks: None,
                    entities: None,
                    entity_table_rows: None,
                    sparse_set: (T::Storage::STORAGE_TYPE == StorageType::SparseSet)
                        .then(|| world.storages().sparse_sets.get(state.component_id).unwrap()),
                    marker: PhantomData,
                    last_change_tick,
                    change_tick,
                }
            }

            unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
                self.table_ticks = Some(table.get_column(state.component_id).unwrap().get_ticks());
            }

            unsafe fn set_archetype(&mut self, state: &Self::State, archetype: &'w Archetype, tables: &'w Tables) {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => {
                        self.entity_table_rows = Some(archetype.entity_table_rows());
                        let table = &tables[archetype.table_id()];
                        self.table_ticks = Some(table.get_column(state.component_id).unwrap().get_ticks());
                    }
                    StorageType::SparseSet => self.entities = Some(archetype.entities()),
                }
            }

            unsafe fn table_fetch(&mut self, table_row: usize) -> bool {
                $is_detected(&*(&*self.table_ticks.unwrap_or_else(|| debug_checked_unreachable()).get_unchecked(table_row)).get(), self.last_change_tick, self.change_tick)
            }

            unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> bool {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => {
                        let table_row = *self.entity_table_rows.unwrap_or_else(|| debug_checked_unreachable()).get_unchecked(archetype_index);
                        $is_detected(&*(&*self.table_ticks.unwrap_or_else(|| debug_checked_unreachable()).get_unchecked(table_row)).get(), self.last_change_tick, self.change_tick)
                    }
                    StorageType::SparseSet => {
                        let entity = *self.entities.unwrap_or_else(|| debug_checked_unreachable()).get_unchecked(archetype_index);
                        let ticks = self.sparse_set.unwrap_or_else(|| debug_checked_unreachable()).get_ticks(entity).cloned().unwrap();
                        $is_detected(&ticks, self.last_change_tick, self.change_tick)
                    }
                }
            }
        }

        /// SAFETY: read-only access
        unsafe impl<'w, T: Component> ReadOnlyFetch<'w, '_> for $fetch_name<'w, T> {}
    };
}

impl_tick_filter!(
    /// Filter that retrieves components of type `T` that have been added since the last execution
    /// of this system.
    ///
    /// This filter is useful to do one-time post-processing on components.
    ///
    /// Because the ordering of systems can change and this filter is only effective on changes
    /// before the query executes you need to use explicit dependency ordering or ordered stages to
    /// avoid frame delays.
    ///
    /// If instead behavior is meant to change on whether the component changed or not
    /// [`ChangeTrackers`](crate::query::ChangeTrackers) may be used.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::component::Component;
    /// # use bevy_ecs::query::Added;
    /// # use bevy_ecs::system::IntoSystem;
    /// # use bevy_ecs::system::Query;
    /// #
    /// # #[derive(Component, Debug)]
    /// # struct Name {};
    ///
    /// fn print_add_name_component(query: Query<&Name, Added<Name>>) {
    ///     for name in query.iter() {
    ///         println!("Named entity created: {:?}", name)
    ///     }
    /// }
    ///
    /// # bevy_ecs::system::assert_is_system(print_add_name_component);
    /// ```
    Added,
    /// The [`FetchState`] of [`Added`].
    AddedState,
    /// The [`Fetch`] of [`Added`].
    AddedFetch,
    ComponentTicks::is_added
);

impl_tick_filter!(
    /// Filter that retrieves components of type `T` that have been changed since the last
    /// execution of this system.
    ///
    /// This filter is useful for synchronizing components, and as a performance optimization as it
    /// means that the query contains fewer items for a system to iterate over.
    ///
    /// Because the ordering of systems can change and this filter is only effective on changes
    /// before the query executes you need to use explicit dependency ordering or ordered
    /// stages to avoid frame delays.
    ///
    /// If instead behavior is meant to change on whether the component changed or not
    /// [`ChangeTrackers`](crate::query::ChangeTrackers) may be used.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::component::Component;
    /// # use bevy_ecs::query::Changed;
    /// # use bevy_ecs::system::IntoSystem;
    /// # use bevy_ecs::system::Query;
    /// #
    /// # #[derive(Component, Debug)]
    /// # struct Name {};
    /// # #[derive(Component)]
    /// # struct Transform {};
    ///
    /// fn print_moving_objects_system(query: Query<&Name, Changed<Transform>>) {
    ///     for name in query.iter() {
    ///         println!("Entity Moved: {:?}", name);
    ///     }
    /// }
    ///
    /// # bevy_ecs::system::assert_is_system(print_moving_objects_system);
    /// ```
    Changed,
    /// The [`FetchState`] of [`Changed`].
    ChangedState,
    /// The [`Fetch`] of [`Changed`].
    ChangedFetch,
    ComponentTicks::is_changed
);
