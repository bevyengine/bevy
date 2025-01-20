use crate::{
    archetype::Archetype,
    component::{ComponentId, Components, Tick},
    entity::Entity,
    query::FilteredAccess,
    storage::{Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use variadics_please::all_tuples;

/// Types that can be used as parameters in a [`Query`].
/// Types that implement this should also implement either [`QueryData`] or [`QueryFilter`]
///
/// # Safety
///
/// Implementor must ensure that
/// [`update_component_access`], [`matches_component_set`], [`fetch`] and [`init_fetch`]
/// obey the following:
///
/// - For each component mutably accessed by [`fetch`], [`update_component_access`] should add write access unless read or write access has already been added, in which case it should panic.
/// - For each component readonly accessed by [`fetch`], [`update_component_access`] should add read access unless write access has already been added, in which case it should panic.
/// - If `fetch` mutably accesses the same component twice, [`update_component_access`] should panic.
/// - [`update_component_access`] may not add a `Without` filter for a component unless [`matches_component_set`] always returns `false` when the component set contains that component.
/// - [`update_component_access`] may not add a `With` filter for a component unless [`matches_component_set`] always returns `false` when the component set doesn't contain that component.
/// - In cases where the query represents a disjunction (such as an `Or` filter) where each element is a valid [`WorldQuery`], the following rules must be obeyed:
///     - [`matches_component_set`] must be a disjunction of the element's implementations
///     - [`update_component_access`] must replace the filters with a disjunction of filters
///     - Each filter in that disjunction must be a conjunction of the corresponding element's filter with the previous `access`
/// - For each resource readonly accessed by [`init_fetch`], [`update_component_access`] should add read access.
/// - Mutable resource access is not allowed.
///
/// When implementing [`update_component_access`], note that `add_read` and `add_write` both also add a `With` filter, whereas `extend_access` does not change the filters.
///
/// [`fetch`]: Self::fetch
/// [`init_fetch`]: Self::init_fetch
/// [`matches_component_set`]: Self::matches_component_set
/// [`Query`]: crate::system::Query
/// [`update_component_access`]: Self::update_component_access
/// [`QueryData`]: crate::query::QueryData
/// [`QueryFilter`]: crate::query::QueryFilter
pub unsafe trait WorldQuery {
    /// The item returned by this [`WorldQuery`]
    /// For `QueryData` this will be the item returned by the query.
    /// For `QueryFilter` this will be either `()`, or a `bool` indicating whether the entity should be included
    /// or a tuple of such things.
    type Item<'a>;

    /// Per archetype/table state used by this [`WorldQuery`] to fetch [`Self::Item`](WorldQuery::Item)
    type Fetch<'a>: Clone;

    /// State used to construct a [`Self::Fetch`](WorldQuery::Fetch). This will be cached inside [`QueryState`](crate::query::QueryState),
    /// so it is best to move as much data / computation here as possible to reduce the cost of
    /// constructing [`Self::Fetch`](WorldQuery::Fetch).
    type State: Send + Sync + Sized;

    /// This function manually implements subtyping for the query items.
    fn shrink<'wlong: 'wshort, 'wshort>(item: Self::Item<'wlong>) -> Self::Item<'wshort>;

    /// This function manually implements subtyping for the query fetches.
    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort>;

    /// Creates a new instance of this fetch.
    /// Readonly accesses resources registered in [`WorldQuery::update_component_access`].
    ///
    /// # Safety
    ///
    /// - `state` must have been initialized (via [`WorldQuery::init_state`]) using the same `world` passed
    ///   in to this function.
    /// - `world` must have the **right** to access any access registered in `update_component_access`.
    /// - There must not be simultaneous resource access conflicting with readonly resource access registered in [`WorldQuery::update_component_access`].
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
    /// - `state` must be the [`State`](Self::State) that `fetch` was initialized with.
    unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table);

    /// Sets available accesses for implementors with dynamic access such as [`FilteredEntityRef`](crate::world::FilteredEntityRef)
    /// or [`FilteredEntityMut`](crate::world::FilteredEntityMut).
    ///
    /// Called when constructing a [`QueryLens`](crate::system::QueryLens) or calling [`QueryState::from_builder`](super::QueryState::from_builder)
    fn set_access(_state: &mut Self::State, _access: &FilteredAccess<ComponentId>) {}

    /// Fetch [`Self::Item`](`WorldQuery::Item`) for either the given `entity` in the current [`Table`],
    /// or for the given `entity` in the current [`Archetype`]. This must always be called after
    /// [`WorldQuery::set_table`] with a `table_row` in the range of the current [`Table`] or after
    /// [`WorldQuery::set_archetype`]  with an `entity` in the current archetype.
    /// Accesses components registered in [`WorldQuery::update_component_access`].
    ///
    /// # Safety
    ///
    /// - Must always be called _after_ [`WorldQuery::set_table`] or [`WorldQuery::set_archetype`]. `entity` and
    ///   `table_row` must be in the range of the current table and archetype.
    /// - There must not be simultaneous conflicting component access registered in `update_component_access`.
    unsafe fn fetch<'w>(
        fetch: &mut Self::Fetch<'w>,
        entity: Entity,
        table_row: TableRow,
    ) -> Self::Item<'w>;

    /// Adds any component accesses used by this [`WorldQuery`] to `access`.
    ///
    /// Used to check which queries are disjoint and can run in parallel
    // This does not have a default body of `{}` because 99% of cases need to add accesses
    // and forgetting to do so would be unsound.
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>);

    /// Creates and initializes a [`State`](WorldQuery::State) for this [`WorldQuery`] type.
    fn init_state(world: &mut World) -> Self::State;

    /// Attempts to initialize a [`State`](WorldQuery::State) for this [`WorldQuery`] type using read-only
    /// access to [`Components`].
    fn get_state(components: &Components) -> Option<Self::State>;

    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    ///
    /// Used to check which [`Archetype`]s can be skipped by the query
    /// (if none of the [`Component`](crate::component::Component)s match)
    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool;
}

macro_rules! impl_tuple_world_query {
    ($(#[$meta:meta])* $(($name: ident, $state: ident)),*) => {

        #[expect(
            clippy::allow_attributes,
            reason = "This is a tuple-related macro; as such the lints below may not always apply."
        )]
        #[allow(
            non_snake_case,
            reason = "The names of some variables are provided by the macro's caller, not by us."
        )]
        #[allow(
            unused_variables,
            reason = "Zero-length tuples won't use any of the parameters."
        )]
        #[allow(
            clippy::unused_unit,
            reason = "Zero-length tuples will generate some function bodies equivalent to `()`; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
        )]
        $(#[$meta])*
        /// SAFETY:
        /// `fetch` accesses are the conjunction of the subqueries' accesses
        /// This is sound because `update_component_access` adds accesses according to the implementations of all the subqueries.
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

            fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
                let ($($name,)*) = fetch;
                ($(
                    $name::shrink_fetch($name),
                )*)
            }

            #[inline]
            unsafe fn init_fetch<'w>(world: UnsafeWorldCell<'w>, state: &Self::State, last_run: Tick, this_run: Tick) -> Self::Fetch<'w> {
                let ($($name,)*) = state;
                // SAFETY: The invariants are uphold by the caller.
                ($(unsafe { $name::init_fetch(world, $name, last_run, this_run) },)*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;

            #[inline]
            unsafe fn set_archetype<'w>(
                fetch: &mut Self::Fetch<'w>,
                state: &Self::State,
                archetype: &'w Archetype,
                table: &'w Table
            ) {
                let ($($name,)*) = fetch;
                let ($($state,)*) = state;
                // SAFETY: The invariants are uphold by the caller.
                $(unsafe { $name::set_archetype($name, $state, archetype, table); })*
            }

            #[inline]
            unsafe fn set_table<'w>(fetch: &mut Self::Fetch<'w>, state: &Self::State, table: &'w Table) {
                let ($($name,)*) = fetch;
                let ($($state,)*) = state;
                // SAFETY: The invariants are uphold by the caller.
                $(unsafe { $name::set_table($name, $state, table); })*
            }

            #[inline(always)]
            unsafe fn fetch<'w>(
                fetch: &mut Self::Fetch<'w>,
                entity: Entity,
                table_row: TableRow
            ) -> Self::Item<'w> {
                let ($($name,)*) = fetch;
                // SAFETY: The invariants are uphold by the caller.
                ($(unsafe { $name::fetch($name, entity, table_row) },)*)
            }

            fn update_component_access(state: &Self::State, access: &mut FilteredAccess<ComponentId>) {
                let ($($name,)*) = state;
                $($name::update_component_access($name, access);)*
            }
            fn init_state(world: &mut World) -> Self::State {
                ($($name::init_state(world),)*)
            }
            fn get_state(components: &Components) -> Option<Self::State> {
                Some(($($name::get_state(components)?,)*))
            }

            fn matches_component_set(state: &Self::State, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
                let ($($name,)*) = state;
                true $(&& $name::matches_component_set($name, set_contains_id))*
            }
        }
    };
}

all_tuples!(
    #[doc(fake_variadic)]
    impl_tuple_world_query,
    0,
    15,
    F,
    S
);
