use core::{mem, ops::Range};

use crate::{
    archetype::{Archetype, ArchetypeEntity},
    change_detection::Tick,
    component::{ComponentId, Components},
    entity::Entity,
    query::FilteredAccess,
    storage::{Table, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
use nonmax::NonMaxU32;
use variadics_please::all_tuples;

/// Types that can be used as parameters in a [`Query`].
/// Types that implement this should also implement either [`QueryData`] or [`QueryFilter`]
///
/// # Safety
///
/// Implementor must ensure that
/// [`update_component_access`], [`QueryData::provide_extra_access`], [`matches_component_set`], [`QueryData::fetch`], [`matches`] and [`init_fetch`]
/// obey the following:
///
/// - For each component mutably accessed by [`QueryData::fetch`], [`update_component_access`] or [`QueryData::provide_extra_access`] should add write access unless read or write access has already been added, in which case it should panic.
/// - For each component readonly accessed by [`QueryData::fetch`] or [`matches`], [`update_component_access`] or [`QueryData::provide_extra_access`] should add read access unless write access has already been added, in which case it should panic.
/// - If `fetch` mutably accesses the same component twice, [`update_component_access`] should panic.
/// - [`update_component_access`] may not add a `Without` filter for a component unless [`matches_component_set`] always returns `false` when the component set contains that component.
/// - [`update_component_access`] may not add a `With` filter for a component unless [`matches_component_set`] always returns `false` when the component set doesn't contain that component.
/// - In cases where the query represents a disjunction (such as an `Or` filter) where each element is a valid [`WorldQuery`], the following rules must be obeyed:
///     - [`matches_component_set`] must be a disjunction of the element's implementations
///     - [`update_component_access`] must replace the filters with a disjunction of filters
///     - Each filter in that disjunction must be a conjunction of the corresponding element's filter with the previous `access`
/// - For each resource readonly accessed by [`init_fetch`], [`update_component_access`] should add read access.
/// - Mutable resource access is not allowed.
/// - Any access added during [`QueryData::provide_extra_access`] must be a subset of `available_access`, and must not conflict with any access in `access`.
///
/// When implementing [`update_component_access`], note that `add_read` and `add_write` both also add a `With` filter, whereas `extend_access` does not change the filters.
///
/// If [`IS_ARCHETYPAL`] is `true`, [`matches`] must return `true` for all inputs.
///
/// When implementing [`find_table_chunk`] and [`find_archetype_chunk`], their return values must be
/// _well formed_ subsets of `rows` and `indices` respectively, even if returning an empty range.
/// What this means is that given a range `(start..end)`, the returned range `(chunk_start..chunk_end)`
/// must satisfy `start <= chunk_start <= chunk_end <= end`. Additionally, they must match the behavior
/// of [`matches`], i.e. calling that method on any row/index in the returned range must return `true`.
///
/// [`find_table_chunk`], [`find_archetype_chunk`], and [`matches`] must not mutably access any world data.
///
/// [`QueryData::provide_extra_access`]: crate::query::QueryData::provide_extra_access
/// [`QueryData::fetch`]: crate::query::QueryData::fetch
/// [`init_fetch`]: Self::init_fetch
/// [`matches_component_set`]: Self::matches_component_set
/// [`Query`]: crate::system::Query
/// [`update_component_access`]: Self::update_component_access
/// [`QueryData`]: crate::query::QueryData
/// [`QueryFilter`]: crate::query::QueryFilter
/// [`find_table_chunk`]: crate::query::WorldQuery::find_table_chunk
/// [`find_archetype_chunk`]: crate::query::WorldQuery::find_archetype_chunk
/// [`matches`]: crate::query::WorldQuery::matches
/// [`IS_ARCHETYPAL`]: crate::query::WorldQuery::IS_ARCHETYPAL
pub unsafe trait WorldQuery {
    /// Per archetype/table state retrieved by this [`WorldQuery`] to compute [`Self::Item`](crate::query::QueryData::Item) for each entity.
    type Fetch<'w>: Clone;

    /// State used to construct a [`Self::Fetch`](WorldQuery::Fetch). This will be cached inside [`QueryState`](crate::query::QueryState),
    /// so it is best to move as much data / computation here as possible to reduce the cost of
    /// constructing [`Self::Fetch`](WorldQuery::Fetch).
    type State: Send + Sync + Sized;

    /// This function manually implements subtyping for the query fetches.
    fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort>;

    /// Creates a new instance of [`Self::Fetch`](WorldQuery::Fetch),
    /// by combining data from the [`World`] with the cached [`Self::State`](WorldQuery::State).
    /// Readonly accesses resources registered in [`WorldQuery::update_component_access`].
    ///
    /// # Safety
    ///
    /// - `state` must have been initialized (via [`WorldQuery::init_state`]) using the same `world` passed
    ///   in to this function.
    /// - `world` must have the **right** to access any access registered in `update_component_access`.
    /// - There must not be simultaneous resource access conflicting with readonly resource access registered in [`WorldQuery::update_component_access`].
    unsafe fn init_fetch<'w, 's>(
        world: UnsafeWorldCell<'w>,
        state: &'s Self::State,
        last_run: Tick,
        this_run: Tick,
    ) -> Self::Fetch<'w>;

    /// Returns true if (and only if) every table of every archetype matched by this fetch contains
    /// all of the matched components.
    ///
    /// This is used to select a more efficient "table iterator"
    /// for "dense" queries. If this returns true, [`WorldQuery::set_table`] must be used before
    /// [`QueryData::fetch`](crate::query::QueryData::fetch) can be called for iterators. If this returns false,
    /// [`WorldQuery::set_archetype`] must be used before [`QueryData::fetch`](crate::query::QueryData::fetch) can be called for
    /// iterators.
    const IS_DENSE: bool;

    /// Returns true if (and only if) this query term relies strictly on archetypes to limit which
    /// entities are accessed by the Query.
    ///
    /// This enables optimizations for [`QueryIter`](`crate::query::QueryIter`) that rely on knowing exactly how
    /// many elements are being iterated (such as `Iterator::collect()`).
    ///
    /// If this is `true`, then [`WorldQuery::matches`] must always return `true`.
    const IS_ARCHETYPAL: bool;

    /// Adjusts internal state to account for the next [`Archetype`]. This will always be called on
    /// archetypes that match this [`WorldQuery`].
    ///
    /// # Safety
    ///
    /// - `archetype` and `tables` must be from the same [`World`] that [`WorldQuery::init_state`] was called on.
    /// - `table` must correspond to `archetype`.
    /// - `state` must be the [`State`](Self::State) that `fetch` was initialized with.
    unsafe fn set_archetype<'w, 's>(
        fetch: &mut Self::Fetch<'w>,
        state: &'s Self::State,
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
    unsafe fn set_table<'w, 's>(
        fetch: &mut Self::Fetch<'w>,
        state: &'s Self::State,
        table: &'w Table,
    );

    /// Adds any component accesses used by this [`WorldQuery`] to `access`.
    ///
    /// Used to check which queries are disjoint and can run in parallel
    // This does not have a default body of `{}` because 99% of cases need to add accesses
    // and forgetting to do so would be unsound.
    fn update_component_access(state: &Self::State, access: &mut FilteredAccess);

    /// Creates and initializes a [`State`](WorldQuery::State) for this [`WorldQuery`] type.
    fn init_state(world: &mut World) -> Self::State;

    /// Attempts to initialize a [`State`](WorldQuery::State) for this [`WorldQuery`] type using read-only
    /// access to [`Components`].
    fn get_state(components: &Components) -> Option<Self::State>;

    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    ///
    /// Used to check which [`Archetype`]s can be skipped by the query
    /// (if none of the [`Component`](crate::component::Component)s match).
    /// This is how archetypal query filters like `With` work.
    fn matches_component_set(
        state: &Self::State,
        set_contains_id: &impl Fn(ComponentId) -> bool,
    ) -> bool;

    /// Searches `rows` for the next contiguous range of indices
    /// this `WorldQuery` matches.
    ///
    /// This can be used to speed up query iteration for custom [`WorldQuery`]
    /// implementations, for example by using an acceleration structure
    /// to seek ahead in the query for the next valid entity.
    ///
    /// The default implementation will simply defer to [`WorldQuery::matches`],
    /// calling the method for each row until it finds the next whole chunk.
    /// Custom implementations should match this behavior, i.e. calling [`WorldQuery::matches`]
    /// on any row in the returned range should return `true`.
    ///
    /// # Safety
    /// - Must always be called _after_ [`WorldQuery::set_table`].
    /// - `rows` must be in the range of the current table.
    #[inline]
    unsafe fn find_table_chunk(
        state: &Self::State,
        fetch: &mut Self::Fetch<'_>,
        table_entities: &[Entity],
        rows: Range<u32>,
    ) -> Range<u32> {
        if Self::IS_ARCHETYPAL {
            rows
        } else {
            rows.find_chunk(|index| {
                let table_row = TableRow::new(index);
                // SAFETY: caller guarantees `indices` is in range of `table`
                let entity = unsafe { *table_entities.get_unchecked(index.get() as usize) };
                // SAFETY: invariants upheld by caller
                unsafe { Self::matches(state, fetch, entity, table_row) }
            })
        }
    }

    /// Searches `indices` for the next contiguous range of indices
    /// this `WorldQuery` matches.
    ///
    /// This can be used to speed up query iteration for custom [`WorldQuery`]
    /// implementations, for example by using an acceleration structure
    /// to seek ahead in the query for the next valid entity.
    ///
    /// The default implementation will simply defer to [`WorldQuery::matches`],
    /// calling the method for each row until it finds the next whole chunk.
    /// Custom implementations should match this behavior, i.e. calling [`WorldQuery::matches`]
    /// on any index in the returned range should return `true`.
    ///
    /// # Safety
    /// - Must always be called _after_ [`WorldQuery::set_archetype`].
    /// - `indices` must be in the range of the current archetype.
    #[inline]
    unsafe fn find_archetype_chunk(
        state: &Self::State,
        fetch: &mut Self::Fetch<'_>,
        archetype_entities: &[ArchetypeEntity],
        indices: Range<u32>,
    ) -> Range<u32> {
        if Self::IS_ARCHETYPAL {
            indices
        } else {
            indices.find_chunk(|index| {
                // SAFETY: caller guarantees `indices` is in range of `archetype`
                let archetype_entity =
                    unsafe { archetype_entities.get_unchecked(index.get() as usize) };

                // SAFETY: invariants upheld by caller
                unsafe {
                    Self::matches(
                        state,
                        fetch,
                        archetype_entity.id(),
                        archetype_entity.table_row(),
                    )
                }
            })
        }
    }

    /// Returns true if the provided [`Entity`] and [`TableRow`] should be included in the query results.
    /// If false, the entity will be skipped.
    ///
    /// Note that this is called after already restricting the matched [`Table`]s and [`Archetype`]s to the
    /// ones that are compatible with the Filter's access.
    ///
    /// Implementors of this method will generally either have a trivial `true` body (required for archetypal filters),
    /// or access the necessary data within this function to make the final decision on filter inclusion.
    ///
    /// # Safety
    ///
    /// Must always be called _after_ [`WorldQuery::set_table`] or [`WorldQuery::set_archetype`]. `entity` and
    /// `table_row` must be in the range of the current table and archetype.
    unsafe fn matches(
        state: &Self::State,
        fetch: &mut Self::Fetch<'_>,
        entity: Entity,
        table_row: TableRow,
    ) -> bool;
}

/// Extension trait for `Range`, adding convenience methods for use with [`WorldQuery::find_table_chunk`]
/// and [`WorldQuery::find_archetype_chunk`].
pub trait RangeExt {
    /// Returns the union of `self` and `other` if they overlap, otherwise returns
    /// whichever is first.
    fn union_or_first(self, other: Self) -> Self;
    /// Returns the next contiguous segment for which `func` returns true for all
    /// indices in the segment.
    fn find_chunk<F: FnMut(NonMaxU32) -> bool>(self, func: F) -> Self;
}

impl RangeExt for Range<u32> {
    #[inline]
    fn union_or_first(mut self, mut other: Self) -> Self {
        if self.start > other.start {
            mem::swap(&mut self, &mut other);
        }

        if self.end >= other.start && self.end < other.end {
            self.end = other.end;
        }

        self
    }

    #[inline]
    fn find_chunk<F: FnMut(NonMaxU32) -> bool>(mut self, mut func: F) -> Self {
        let mut index = self.start;
        while index < self.end {
            // SAFETY: index is taken from an exclusive range, so it can't be max
            let nonmax_index = unsafe { NonMaxU32::new_unchecked(index) };
            let matches = func(nonmax_index);
            if matches {
                break;
            }
            index = index.wrapping_add(1);
        }
        self.start = index;

        while index < self.end {
            // SAFETY: index is taken from an exclusive range, so it can't be max
            let nonmax_index = unsafe { NonMaxU32::new_unchecked(index) };
            let matches = func(nonmax_index);
            if !matches {
                break;
            }
            index = index.wrapping_add(1);
        }
        self.end = index;

        self
    }
}

/// A wrapper `Fetch` type for tuple query terms that caches the last seen valid chunk.
#[doc(hidden)]
pub struct TupleFetch<'w, T: WorldQuery> {
    /// the `Fetch` type of the query term
    pub(crate) fetch: T::Fetch<'w>,
    // the `end` value of the chunk last found in `find_table_chunk`/`find_archetype_chunk`
    chunk_end: u32,
}

impl<'w, T: WorldQuery> Clone for TupleFetch<'w, T> {
    fn clone(&self) -> Self {
        Self {
            fetch: self.fetch.clone(),
            chunk_end: self.chunk_end,
        }
    }
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
        #[allow(
            unused_mut,
            reason = "Zero-length tuples will generate some function bodies that don't mutate their parameters; however, this macro is meant for all applicable tuples, and as such it makes no sense to rewrite it just for that case."
        )]
        $(#[$meta])*
        /// SAFETY:
        /// `fetch` accesses are the conjunction of the subqueries' accesses
        /// This is sound because `update_component_access` adds accesses according to the implementations of all the subqueries.
        /// `update_component_access` adds all `With` and `Without` filters from the subqueries.
        /// This is sound because `matches_component_set` always returns `false` if any the subqueries' implementations return `false`.
        /// `lookahead_table` and `lookahead_archetype` always return a subset of `rows`/`indices`
        /// This is sound because each `lookahead_table` and `lookahead_archetype` each
        unsafe impl<$($name: WorldQuery),*> WorldQuery for ($($name,)*) {
            type Fetch<'w> = ($(TupleFetch<'w, $name>,)*);
            type State = ($($name::State,)*);


            fn shrink_fetch<'wlong: 'wshort, 'wshort>(fetch: Self::Fetch<'wlong>) -> Self::Fetch<'wshort> {
                let ($($name,)*) = fetch;
                ($(
                    TupleFetch {
                        fetch: $name::shrink_fetch($name.fetch),
                        chunk_end: $name.chunk_end
                    },
                )*)
            }

            #[inline]
            unsafe fn init_fetch<'w, 's>(world: UnsafeWorldCell<'w>, state: &'s Self::State, last_run: Tick, this_run: Tick) -> Self::Fetch<'w> {
                let ($($name,)*) = state;
                ($(
                    TupleFetch {
                        // SAFETY: The invariants are upheld by the caller.
                        fetch: unsafe { $name::init_fetch(world, $name, last_run, this_run) },
                        chunk_end: 0,
                    },
                )*)
            }

            const IS_DENSE: bool = true $(&& $name::IS_DENSE)*;
            const IS_ARCHETYPAL: bool = true $(&& $name::IS_ARCHETYPAL)*;

            #[inline]
            unsafe fn set_archetype<'w, 's>(
                fetch: &mut Self::Fetch<'w>,
                state: &'s Self::State,
                archetype: &'w Archetype,
                table: &'w Table
            ) {
                let ($($name,)*) = fetch;
                let ($($state,)*) = state;
                $({
                    // SAFETY: The invariants are upheld by the caller.
                    unsafe { $name::set_archetype(&mut $name.fetch, $state, archetype, table); };
                    $name.chunk_end = 0;
                })*
            }

            #[inline]
            unsafe fn set_table<'w, 's>(fetch: &mut Self::Fetch<'w>, state: &'s Self::State, table: &'w Table) {
                let ($($name,)*) = fetch;
                let ($($state,)*) = state;
                $({
                    // SAFETY: The invariants are upheld by the caller.
                    unsafe { $name::set_table(&mut $name.fetch, $state, table); }
                    $name.chunk_end = 0;
                })*
            }


            fn update_component_access(state: &Self::State, access: &mut FilteredAccess) {
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

            #[inline]
            unsafe fn find_table_chunk(
                state: &Self::State,
                fetch: &mut Self::Fetch<'_>,
                table_entities: &[Entity],
                mut rows: Range<u32>,
            ) -> Range<u32> {
                if Self::IS_ARCHETYPAL || rows.is_empty() {
                    rows
                } else {
                    let mut chunk = rows.clone();
                    let ($($name,)*) = fetch;
                    let ($($state,)*) = state;
                    loop {
                        let mut any_valid_terms = false;
                        $({
                            let mut term_chunk = chunk.start..$name.chunk_end;
                            if term_chunk.is_empty() {
                                // SAFETY: chunk.start..rows.end is always a subset of `rows`
                                term_chunk = unsafe { $name::find_table_chunk($state, &mut $name.fetch, table_entities, chunk.start..rows.end) };
                            }
                            any_valid_terms |= !term_chunk.is_empty();
                            chunk.start = term_chunk.start;
                            chunk.end = chunk.end.min(term_chunk.end);
                            $name.chunk_end = term_chunk.end;
                        })*

                        if !chunk.is_empty() || !any_valid_terms {
                            break;
                        }
                    }
                    chunk
                }
            }

            #[inline]
            unsafe fn find_archetype_chunk(
                state: &Self::State,
                fetch: &mut Self::Fetch<'_>,
                archetype_entities: &[ArchetypeEntity],
                mut indices: Range<u32>,
            ) -> Range<u32> {
                if Self::IS_ARCHETYPAL || indices.is_empty() {
                    indices
                } else {
                    let mut chunk = indices.clone();
                    let ($($name,)*) = fetch;
                    let ($($state,)*) = state;
                    // SAFETY: `indices` is only ever narrowed as we iterate subqueries, so it's
                    // always valid to pass to the next term. Other invariants are upheld by
                    // the caller.
                    $(indices = unsafe { $name::find_archetype_chunk($state, &mut $name.fetch, archetype_entities, indices) };)*
                    indices
                }
            }

            #[inline]
            unsafe fn matches(
                state: &Self::State,
                fetch: &mut Self::Fetch<'_>,
                entity: Entity,
                table_row: TableRow,
            ) -> bool {
                if Self::IS_ARCHETYPAL {
                    true
                } else {
                    let ($($name,)*) = fetch;
                    let ($($state,)*) = state;
                    // SAFETY: invariants are upheld by the caller.
                    true $(&& unsafe { $name::matches($state, &mut $name.fetch, entity, table_row) })*
                }
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
