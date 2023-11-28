use crate::{
    archetype::{Archetype, ArchetypeComponentId},
    component::{ComponentId, Tick},
    entity::Entity,
    query::{Access, FilteredAccess},
    storage::{Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use bevy_utils::all_tuples;

/// Types that can be used as parameters in a [`Query`].
/// Types that implement this should also implement either [`WorldQueryData`] or [`WorldQueryFilter`]
///
/// # Safety
///
/// Implementor must ensure that
/// [`update_component_access`], [`update_archetype_component_access`], [`matches_component_set`], and [`fetch`]
/// obey the following:
///
/// - For each component mutably accessed by [`fetch`], [`update_component_access`] should add write access unless read or write access has already been added, in which case it should panic.
/// - For each component readonly accessed by [`fetch`], [`update_component_access`] should add read access unless write access has already been added, in which case it should panic.
/// - For each component mutably accessed by [`fetch`], [`update_archetype_component_access`] should add write access if that component belongs to the archetype.
/// - For each component readonly accessed by [`fetch`], [`update_archetype_component_access`] should add read access if that component belongs to the archetype.
/// - If `fetch` mutably accesses the same component twice, [`update_component_access`] should panic.
/// - [`update_component_access`] may not add a `Without` filter for a component unless [`matches_component_set`] always returns `false` when the component set contains that component.
/// - [`update_component_access`] may not add a `With` filter for a component unless [`matches_component_set`] always returns `false` when the component set doesn't contain that component.
/// - In cases where the query represents a disjunction (such as an `Or` filter) where each element is a valid [`WorldQuery`], the following rules must be obeyed:
///     - [`matches_component_set`] must be a disjunction of the element's implementations
///     - [`update_component_access`] must replace the filters with a disjunction of filters
///     - Each filter in that disjunction must be a conjunction of the corresponding element's filter with the previous `access`
///
/// When implementing [`update_component_access`], note that `add_read` and `add_write` both also add a `With` filter, whereas `extend_access` does not change the filters.
///
/// [`fetch`]: Self::fetch
/// [`matches_component_set`]: Self::matches_component_set
/// [`Query`]: crate::system::Query
/// [`update_archetype_component_access`]: Self::update_archetype_component_access
/// [`update_component_access`]: Self::update_component_access
/// [`WorldQueryData`]: crate::query::WorldQueryData
/// [`WorldQueryFilter`]: crate::query::WorldQueryFilter
pub unsafe trait WorldQuery {
    /// The item returned by this [`WorldQuery`]
    /// For `WorldQueryData` this will be the item returned by the query.
    /// For `WorldQueryFilter` this will be either `()`, or a `bool` indicating whether the entity should be included
    /// or a tuple of such things.
    type Item<'a>;

    /// Per archetype/table state used by this [`WorldQuery`] to fetch [`Self::Item`](crate::query::WorldQuery::Item)
    type Fetch<'a>: Clone;

    /// State used to construct a [`Self::Fetch`](crate::query::WorldQuery::Fetch). This will be cached inside [`QueryState`](crate::query::QueryState),
    /// so it is best to move as much data / computation here as possible to reduce the cost of
    /// constructing [`Self::Fetch`](crate::query::WorldQuery::Fetch).
    type State: Send + Sync + Sized;

    /// This function manually implements subtyping for the query items.
    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort>;

    /// Creates a new instance of this fetch.
    ///
    /// # Safety
    ///
    /// - `world` must have permission to access any of the components specified in `Self::update_archetype_component_access`.
    /// - `state` must have been initialized (via [`WorldQuery::init_state`]) using the same `world` passed
    ///   in to this function.
    unsafe fn init_fetch<'w>(
        world: UnsafeWorldCell<'w>,
        state: &Self::State,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w>;

    /// Returns true if (and only if) every table of every archetype matched by this fetch contains
    /// all of the matched components. This is used to select a more efficient "table iterator"
    /// for "dense" queries. If this returns true, [`WorldQuery::set_table`] must be used before
    /// [`WorldQuery::fetch`] can be called for iterators. If this returns false,
    /// [`WorldQuery::set_archetype`] must be used before [`WorldQuery::fetch`] can be called for
    /// iterators.
    const IS_DENSE: bool;

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`WorldQuery`].
    ///
    /// # Safety
    ///
    /// - `archetype` and `tables` must be from the same [`World`] that [`WorldQuery::init_state`] was called on.
    /// - [`Self::update_archetype_component_access`] must have been previously called with `archetype`.
    /// - `table` must correspond to `archetype`.
    /// - `state` must be the [`State`](Self::State) that `fetch` was initialized with.
    unsafe fn set_archetype<'w>(
        fetch: &mut Self::Fetch<'w>,
        state: &Self::State,
        archetype: &'w Archetype,
        table: &'w Table,
    );

    /// Adjusts internal state to account for the next [`Table`]. This will always be called on tables
    /// that match this [`WorldQuery`].
    ///
    /// # Safety
    ///
    /// - `table` must be from the same [`World`] that [`WorldQuery::init_state`] was called on.
    /// - `table` must belong to an archetype that was previously registered with
    ///   [`Self::update_archetype_component_access`].
    /// - `state` must be the [`State`](Self::State) that `fetch` was initialized with.
    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table);

    /// Fetch [`Self::Item`](`WorldQuery::Item`) for either the given `entity` in the current [`Table`],
    /// or for the given `entity` in the current [`Archetype`]. This must always be called after
    /// [`WorldQuery::set_table`] with a `table_row` in the range of the current [`Table`] or after
    /// [`WorldQuery::set_archetype`]  with a `entity` in the current archetype.
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`WorldQuery::set_table`] or [`WorldQuery::set_archetype`]. `entity` and
    /// `table_row` must be in the range of the current table and archetype.
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w>;

    /// Adds any component accesses used by this [`WorldQuery`] to `access`.
    // This does not have a default body of `{}` because 99% of cases need to add accesses
    // and forgetting to do so would be unsound.
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>);

    /// For the given `archetype`, adds any component accessed used by this [`WorldQuery`] to `access`.
    // This does not have a default body of `{}` because 99% of cases need to add accesses
    // and forgetting to do so would be unsound.
    fn update_archetype_component_access(
        state: &Self::State,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    );

    /// Creates and initializes a [`State`](WorldQuery::State) for this [`WorldQuery`] type.
    fn init_state(world: &mut World) -> Self::State;

    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool;
}

macro_rules! impl_tuple_world_query {
    ($(($name: ident, $state: ident)),*) => {

        #[allow(non_snake_case)]
        #[allow(clippy::unused_unit)]
        /// SAFETY:
        /// `fetch` accesses are the conjunction of the subqueries' accesses
        /// This is sound because `update_component_access` and `update_archetype_component_access` adds accesses according to the implementations of all the subqueries.
        /// `update_component_access` adds all `With` and `Without` filters from the subqueries.
        /// This is sound because `matches_component_set` always returns `false` if any the subqueries' implementations return `false`.
        unsafe impl<$($name: WorldQuery),*> WorldQuery for ($($name,)*) {
            type Fetch<'w> = ($($name::Fetch<'w>,)*);
            type Item<'w> = ($($name::Item<'w>,)*);
            type State = ($($name::State,)*);

            fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort> {
                let ($($name,)*) = item;
                ($(
                    $name::shrink($name),
                )*)
            }

            #[inline]
            #[allow(clippy::unused_unit)]
            unsafe fn init_fetch<'w>(_world: UnsafeWorldCell<'w>, state: &Self::State, _last_run: Tick, _this_run: Tick) -> Self::Fetch<'w> {
                let ($($name,)*) = state;
                ($($name::init_fetch(_world, $name, _last_run, _this_run),)*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            #[inline]
            unsafe fn set_archetype<'w>(
                _fetch: &mut Self::Fetch<'w>,
                _state: &Self::State,
                _archetype: &'w Archetype,
                _table: &'w Table
            ) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $($name::set_archetype($name, $state, _archetype, _table);)*
            }

            #[inline]
            unsafe fn set_table<'w>(_fetch: &mut Self::Fetch<'w>, _state: &Self::State, _table: &'w Table) {
                let ($($name,)*) = _fetch;
                let ($($state,)*) = _state;
                $($name::set_table($name, $state, _table);)*
            }

            #[inline(always)]
            #[allow(clippy::unused_unit)]
            unsafe fn fetch<'w>(
                _fetch: &mut Self::Fetch<'w>,
                _entity: Entity,
                _table_row: TableRow
            ) -> Self::Item<'w> {
                let ($($name,)*) = _fetch;
                ($($name::fetch($name, _entity, _table_row),)*)
            }

            fn update_component_access(state: &Self::State, _access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = state;
                $($name::update_component_access($name, _access);)*
            }

            fn update_archetype_component_access(state: &Self::State, _archetype: &Archetype, _access: &mut Access<ArchetypeComponentId>) {
                let ($($name,)*) = state;
                $($name::update_archetype_component_access($name, _archetype, _access);)*
            }

            fn init_state(_world: &mut World) -> Self::State {
                ($($name::init_state(_world),)*)
            }

            fn matches_component_set(state: &Self::State, _set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($name,)*) = state;
                true $(&& $name::matches_component_set($name, _set_contains_id))*
            }
        }
    };
}

all_tuples!(impl_tuple_world_query, 0, 15, F, S);
