use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{Component, ComponentId, ComponentStorage, ComponentTicks, StorageType},
    entity::Entity,
    query::{
        debug_checked_unreachable, Access, Fetch, FetchState, FilteredAccess, QueryFetch,
        WorldQuery, WorldQueryGats,
    },
    storage::{ComponentSparseSet, Table, Tables},
    world::World,
};
use bevy_ecs_macros::all_tuples;
use bevy_ptr::{ThinSlicePtr, UnsafeCellDeref};
use std::{cell::UnsafeCell, marker::PhantomData};

use super::ReadOnlyWorldQuery;

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

// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for With<T> {
    type ReadOnly = Self;
    type State = WithState<T>;

    #[allow(clippy::semicolon_if_nothing_returned)]
    fn shrink<'wlong: 'wshort, 'wshort>(
        item: super::QueryItem<'wlong, Self>,
    ) -> super::QueryItem<'wshort, Self> {
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

impl<T: Component> FetchState for WithState<T> {
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

impl<T: Component> WorldQueryGats<'_> for With<T> {
    type Fetch = WithFetch<T>;
    type _State = WithState<T>;
}

// SAFETY: no component access or archetype component access
unsafe impl<'w, T: Component> Fetch<'w> for WithFetch<T> {
    type Item = ();
    type State = WithState<T>;

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

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

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
    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) {}

    #[inline]
    unsafe fn table_fetch(&mut self, _table_row: usize) {}

    #[inline]
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        access.add_with(state.component_id);
    }

    #[inline]
    fn update_archetype_component_access(
        _state: &Self::State,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }
}

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> ReadOnlyWorldQuery for With<T> {}

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

// SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
unsafe impl<T: Component> WorldQuery for Without<T> {
    type ReadOnly = Self;
    type State = WithoutState<T>;

    #[allow(clippy::semicolon_if_nothing_returned)]
    fn shrink<'wlong: 'wshort, 'wshort>(
        item: super::QueryItem<'wlong, Self>,
    ) -> super::QueryItem<'wshort, Self> {
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

impl<T: Component> FetchState for WithoutState<T> {
    fn init(world: &mut World) -> Self {
        let component_id = world.init_component::<T>();
        Self {
            component_id,
            marker: PhantomData,
        }
    }

    fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        !set_contains_id(self.component_id)
    }
}

impl<T: Component> WorldQueryGats<'_> for Without<T> {
    type Fetch = WithoutFetch<T>;
    type _State = WithoutState<T>;
}

// SAFETY: no component access or archetype component access
unsafe impl<'w, T: Component> Fetch<'w> for WithoutFetch<T> {
    type Item = ();
    type State = WithoutState<T>;

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

    const IS_DENSE: bool = {
        match T::Storage::STORAGE_TYPE {
            StorageType::Table => true,
            StorageType::SparseSet => false,
        }
    };

    const IS_ARCHETYPAL: bool = true;

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
    unsafe fn archetype_fetch(&mut self, _archetype_index: usize) {}

    #[inline]
    unsafe fn table_fetch(&mut self, _table_row: usize) {}

    #[inline]
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
        access.add_without(state.component_id);
    }

    #[inline]
    fn update_archetype_component_access(
        _state: &Self::State,
        _archetype: &Archetype,
        _access: &mut Access<ArchetypeComponentId>,
    ) {
    }
}

// SAFETY: no component access or archetype component access
unsafe impl<T: Component> ReadOnlyWorldQuery for Without<T> {}

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
#[derive(Clone, Copy)]
#[doc(hidden)]
pub struct OrFetch<'w, T: Fetch<'w>> {
    fetch: T,
    matches: bool,
    _marker: PhantomData<&'w ()>,
}

macro_rules! impl_query_filter_tuple {
    ($(($filter: ident, $state: ident)),*) => {
        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        // SAFETY: defers to soundness of `$filter: WorldQuery` impl
        unsafe impl<$($filter: WorldQuery),*> WorldQuery for Or<($($filter,)*)> {
            type ReadOnly = Or<($($filter::ReadOnly,)*)>;
            type State = Or<($($filter::State,)*)>;

            fn shrink<'wlong: 'wshort, 'wshort>(item: super::QueryItem<'wlong, Self>) -> super::QueryItem<'wshort, Self> {
                item
            }
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<'w, $($filter: WorldQueryGats<'w>),*> WorldQueryGats<'w> for Or<($($filter,)*)> {
            type Fetch = Or<($(OrFetch<'w, QueryFetch<'w, $filter>>,)*)>;
            type _State = Or<($($filter::_State,)*)>;
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        // SAFETY: update_component_access and update_archetype_component_access are called for each item in the tuple
        unsafe impl<'w, $($filter: Fetch<'w>),*> Fetch<'w> for Or<($(OrFetch<'w, $filter>,)*)> {
            type State = Or<($(<$filter as Fetch<'w>>::State,)*)>;
            type Item = bool;

            const IS_DENSE: bool = true $(&& $filter::IS_DENSE)*;

            const IS_ARCHETYPAL: bool = true $(&& $filter::IS_ARCHETYPAL)*;

            unsafe fn init(world: &'w World, state: & Or<($(<$filter as Fetch<'w>>::State,)*)>, last_change_tick: u32, change_tick: u32) -> Self {
                let ($($filter,)*) = &state.0;
                Or(($(OrFetch {
                    fetch: <$filter as Fetch<'w>>::init(world, $filter, last_change_tick, change_tick),
                    matches: false,
                    _marker: PhantomData,
                },)*))
            }

            #[inline]
            unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
                let ($($filter,)*) = &mut self.0;
                let ($($state,)*) = &state.0;
                $(
                    $filter.matches = $state.matches_component_set(&|id| table.has_column(id));
                    if $filter.matches {
                        $filter.fetch.set_table($state, table);
                    }
                )*
            }

            #[inline]
            unsafe fn set_archetype(&mut self, state: & Self::State, archetype: &'w Archetype, tables: &'w Tables) {
                let ($($filter,)*) = &mut self.0;
                let ($($state,)*) = &state.0;
                $(
                    $filter.matches = $state.matches_component_set(&|id| archetype.contains(id));
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

            #[inline]
            unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool {
                self.table_fetch(table_row)
            }

            #[inline]
            unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool {
                self.archetype_fetch(archetype_index)
            }

            fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
                let ($($filter,)*) = &state.0;

                // We do not unconditionally add `$filter`'s `with`/`without` accesses to `access`
                // as this would be unsound. For example the following two queries should conflict:
                // - Query<&mut B, Or<(With<A>, ())>>
                // - Query<&mut B, Without<A>>
                //
                // If we were to unconditionally add `$name`'s `with`/`without` accesses then `Or<(With<A>, ())>`
                // would have a `With<A>` access which is incorrect as this `WorldQuery` will match entities that
                // do not have the `A` component. This is the same logic as the `AnyOf<...>: WorldQuery` impl.
                //
                // The correct thing to do here is to only add a `with`/`without` access to `_access` if all
                // `$filter` params have that `with`/`without` access. More jargony put- we add the intersection
                // of all `with`/`without` accesses of the `$filter` params to `access`.
                let mut _intersected_access = access.clone();
                let mut _not_first = false;
                $(
                    if _not_first {
                        let mut intermediate = access.clone();
                        $filter::update_component_access($filter, &mut intermediate);
                        _intersected_access.extend_intersect_filter(&intermediate);
                        _intersected_access.extend_access(&intermediate);
                    } else {
                        $filter::update_component_access($filter, &mut _intersected_access);
                        _not_first = true;
                    }
                )*

                *access = _intersected_access;
            }

            fn update_archetype_component_access(state: &Self::State, archetype: &Archetype, access: &mut Access<ArchetypeComponentId>) {
                let ($($filter,)*) = &state.0;
                $($filter::update_archetype_component_access($filter, archetype, access);)*
            }
        }

        #[allow(unused_variables)]
        #[allow(non_snake_case)]
        impl<$($filter: FetchState),*> FetchState for Or<($($filter,)*)> {
            fn init(world: &mut World) -> Self {
                Or(($($filter::init(world),)*))
            }

            fn matches_component_set(&self, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($filter,)*) = &self.0;
                false $(|| $filter.matches_component_set(_set_contains_id))*
            }
        }

        // SAFE: filters are read only
        unsafe impl<$($filter: ReadOnlyWorldQuery),*> ReadOnlyWorldQuery for Or<($($filter,)*)> {}
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
            table_ticks: Option<ThinSlicePtr<'w, UnsafeCell<ComponentTicks>>>,
            entity_table_rows: Option<ThinSlicePtr<'w, usize>>,
            marker: PhantomData<T>,
            entities: Option<ThinSlicePtr<'w, Entity>>,
            sparse_set: Option<&'w ComponentSparseSet>,
            last_change_tick: u32,
            change_tick: u32,
        }

        #[doc(hidden)]
        $(#[$state_meta])*
        pub struct $state_name<T> {
            component_id: ComponentId,
            marker: PhantomData<T>,
        }

        // SAFETY: `ROQueryFetch<Self>` is the same as `QueryFetch<Self>`
        unsafe impl<T: Component> WorldQuery for $name<T> {
            type ReadOnly = Self;
            type State = $state_name<T>;

            fn shrink<'wlong: 'wshort, 'wshort>(item: super::QueryItem<'wlong, Self>) -> super::QueryItem<'wshort, Self> {
                item
            }
        }

        impl<T: Component> FetchState for $state_name<T> {
            fn init(world: &mut World) -> Self {
                Self {
                    component_id: world.init_component::<T>(),
                    marker: PhantomData,
                }
            }

            fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                set_contains_id(self.component_id)
            }
        }

        impl<'w, T: Component> WorldQueryGats<'w> for $name<T> {
            type Fetch = $fetch_name<'w, T>;
            type _State = $state_name<T>;
        }

        // SAFETY: this reads the T component. archetype component access and component access are updated to reflect that
        unsafe impl<'w, T: Component> Fetch<'w> for $fetch_name<'w, T> {
            type State = $state_name<T>;
            type Item = bool;

            unsafe fn init(world: &'w World, state: & $state_name<T>, last_change_tick: u32, change_tick: u32) -> Self {
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

            const IS_DENSE: bool = {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => true,
                    StorageType::SparseSet => false,
                }
            };

            const IS_ARCHETYPAL:  bool = false;

            unsafe fn set_table(&mut self, state: &Self::State, table: &'w Table) {
                self.table_ticks = Some(table.get_column(state.component_id).unwrap().get_ticks_slice().into());
            }

            unsafe fn set_archetype(&mut self, state: &Self::State, archetype: &'w Archetype, tables: &'w Tables) {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => {
                        self.entity_table_rows = Some(archetype.entity_table_rows().into());
                        let table = &tables[archetype.table_id()];
                        self.table_ticks = Some(table.get_column(state.component_id).unwrap().get_ticks_slice().into());
                    }
                    StorageType::SparseSet => self.entities = Some(archetype.entities().into()),
                }
            }

            unsafe fn table_fetch(&mut self, table_row: usize) -> bool {
                $is_detected(&*(self.table_ticks.unwrap_or_else(|| debug_checked_unreachable()).get(table_row)).deref(), self.last_change_tick, self.change_tick)
            }

            unsafe fn archetype_fetch(&mut self, archetype_index: usize) -> bool {
                match T::Storage::STORAGE_TYPE {
                    StorageType::Table => {
                        let table_row = *self.entity_table_rows.unwrap_or_else(|| debug_checked_unreachable()).get(archetype_index);
                        $is_detected(&*(self.table_ticks.unwrap_or_else(|| debug_checked_unreachable()).get(table_row)).deref(), self.last_change_tick, self.change_tick)
                    }
                    StorageType::SparseSet => {
                        let entity = *self.entities.unwrap_or_else(|| debug_checked_unreachable()).get(archetype_index);
                        let ticks = self
                            .sparse_set
                            .unwrap_or_else(|| debug_checked_unreachable())
                            .get_ticks(entity)
                            .map(|ticks| &*ticks.get())
                            .cloned()
                            .unwrap();
                        $is_detected(&ticks, self.last_change_tick, self.change_tick)
                    }
                }
            }

            #[inline]
            unsafe fn table_filter_fetch(&mut self, table_row: usize) -> bool {
                self.table_fetch(table_row)
            }

            #[inline]
            unsafe fn archetype_filter_fetch(&mut self, archetype_index: usize) -> bool {
                self.archetype_fetch(archetype_index)
            }

            #[inline]
            fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
                if access.access().has_write(state.component_id) {
                    panic!("$state_name<{}> conflicts with a previous access in this query. Shared access cannot coincide with exclusive access.",
                        std::any::type_name::<T>());
                }
                access.add_read(state.component_id);
            }

            #[inline]
            fn update_archetype_component_access(
                state: &Self::State,
                archetype: &Archetype,
                access: &mut Access<ArchetypeComponentId>,
            ) {
                if let Some(archetype_component_id) = archetype.get_archetype_component_id(state.component_id) {
                    access.add_read(archetype_component_id);
                }
            }
        }

        /// SAFETY: read-only access
        unsafe impl<T: Component> ReadOnlyWorldQuery for $name<T> {}

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

        impl<T> Copy for $fetch_name<'_, T> {}
    };
}

impl_tick_filter!(
    /// A filter on a component that only retains results added after the system last ran.
    ///
    /// A common use for this filter is one-time initialization.
    ///
    /// To retain all results without filtering but still check whether they were added after the
    /// system last ran, use [`ChangeTrackers<T>`](crate::query::ChangeTrackers).
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
    /// A filter on a component that only retains results added or mutably dereferenced after the system last ran.
    ///  
    /// A common use for this filter is avoiding redundant work when values have not changed.
    ///
    /// **Note** that simply *mutably dereferencing* a component is considered a change ([`DerefMut`](std::ops::DerefMut)).
    /// Bevy does not compare components to their previous values.
    ///
    /// To retain all results without filtering but still check whether they were changed after the
    /// system last ran, use [`ChangeTrackers<T>`](crate::query::ChangeTrackers).
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

/// A marker trait to indicate that the filter works at an archetype level.
///
/// This is needed to implement [`ExactSizeIterator`](std::iter::ExactSizeIterator) for
/// [`QueryIter`](crate::query::QueryIter) that contains archetype-level filters.
///
/// The trait must only be implement for filters where its corresponding [`Fetch::IS_ARCHETYPAL`](crate::query::Fetch::IS_ARCHETYPAL)
/// is [`prim@true`]. As such, only the [`With`] and [`Without`] filters can implement the trait.
/// [Tuples](prim@tuple) and [`Or`] filters are automatically implemented with the trait only if its containing types
/// also implement the same trait.
///
/// [`Added`] and [`Changed`] works with entities, and therefore are not archetypal. As such
/// they do not implement [`ArchetypeFilter`].
pub trait ArchetypeFilter {}

impl<T> ArchetypeFilter for With<T> {}
impl<T> ArchetypeFilter for Without<T> {}

macro_rules! impl_archetype_filter_tuple {
    ($($filter: ident),*) => {
        impl<$($filter: ArchetypeFilter),*> ArchetypeFilter for ($($filter,)*) {}

        impl<$($filter: ArchetypeFilter),*> ArchetypeFilter for Or<($($filter,)*)> {}
    };
}

all_tuples!(impl_archetype_filter_tuple, 0, 15, F);
