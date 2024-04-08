use crate::{
    archetype::{Archetype, ArchetypeEntity, Archetypes},
    component::Tick,
    entity::{Entities, Entity},
    query::{ArchetypeFilter, DebugCheckedUnwrap, QueryState, StorageId},
    storage::{Table, TableRow, Tables},
    world::unsafe_world_cell::UnsafeWorldCell,
};
use std::{borrow::Borrow, iter::FusedIterator, mem::MaybeUninit, ops::Range};

use super::{QueryData, QueryFilter, ReadOnlyQueryData};

/// An [`Iterator`] over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::iter`](crate::system::Query::iter) and
/// [`Query::iter_mut`](crate::system::Query::iter_mut) methods.
pub struct QueryIter<'w, 's, D: QueryData, F: QueryFilter> {
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<D, F>,
    cursor: QueryIterationCursor<'w, 's, D, F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter> QueryIter<'w, 's, D, F> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        QueryIter {
            query_state,
            // SAFETY: We only access table data that has been registered in `query_state`.
            tables: unsafe { &world.storages().tables },
            archetypes: world.archetypes(),
            // SAFETY: The invariants are uphold by the caller.
            cursor: unsafe { QueryIterationCursor::init(world, query_state, last_run, this_run) },
        }
    }

    /// Executes the equivalent of [`Iterator::fold`] over a contiguous segment
    /// from an table.
    ///
    /// # Safety
    ///  - all `rows` must be in `[0, table.entity_count)`.
    ///  - `table` must match D and F
    ///  - Both `D::IS_DENSE` and `F::IS_DENSE` must be true.
    #[inline]
    pub(super) unsafe fn fold_over_table_range<B, Func>(
        &mut self,
        mut accum: B,
        func: &mut Func,
        table: &'w Table,
        rows: Range<usize>,
    ) -> B
    where
        Func: FnMut(B, D::Item<'w>) -> B,
    {
        assert!(
            rows.end <= u32::MAX as usize,
            "TableRow is only valid up to u32::MAX"
        );

        D::set_table(&mut self.cursor.fetch, &self.query_state.fetch_state, table);
        F::set_table(
            &mut self.cursor.filter,
            &self.query_state.filter_state,
            table,
        );

        let entities = table.entities();
        for row in rows {
            // SAFETY: Caller assures `row` in range of the current archetype.
            let entity = unsafe { entities.get_unchecked(row) };
            let row = TableRow::from_usize(row);

            // SAFETY: set_table was called prior.
            // Caller assures `row` in range of the current archetype.
            let fetched = unsafe { !F::filter_fetch(&mut self.cursor.filter, *entity, row) };
            if fetched {
                continue;
            }

            // SAFETY: set_table was called prior.
            // Caller assures `row` in range of the current archetype.
            let item = D::fetch(&mut self.cursor.fetch, *entity, row);

            accum = func(accum, item);
        }
        accum
    }

    /// Executes the equivalent of [`Iterator::fold`] over a contiguous segment
    /// from an archetype.
    ///
    /// # Safety
    ///  - all `indices` must be in `[0, archetype.len())`.
    ///  - `archetype` must match D and F
    ///  - Either `D::IS_DENSE` or `F::IS_DENSE` must be false.
    #[inline]
    pub(super) unsafe fn fold_over_archetype_range<B, Func>(
        &mut self,
        mut accum: B,
        func: &mut Func,
        archetype: &'w Archetype,
        indices: Range<usize>,
    ) -> B
    where
        Func: FnMut(B, D::Item<'w>) -> B,
    {
        let table = self.tables.get(archetype.table_id()).debug_checked_unwrap();
        D::set_archetype(
            &mut self.cursor.fetch,
            &self.query_state.fetch_state,
            archetype,
            table,
        );
        F::set_archetype(
            &mut self.cursor.filter,
            &self.query_state.filter_state,
            archetype,
            table,
        );

        let entities = archetype.entities();
        for index in indices {
            // SAFETY: Caller assures `index` in range of the current archetype.
            let archetype_entity = unsafe { entities.get_unchecked(index) };

            // SAFETY: set_archetype was called prior.
            // Caller assures `index` in range of the current archetype.
            let fetched = unsafe {
                !F::filter_fetch(
                    &mut self.cursor.filter,
                    archetype_entity.id(),
                    archetype_entity.table_row(),
                )
            };
            if fetched {
                continue;
            }

            // SAFETY: set_archetype was called prior, `index` is an archetype index in range of the current archetype
            // Caller assures `index` in range of the current archetype.
            let item = unsafe {
                D::fetch(
                    &mut self.cursor.fetch,
                    archetype_entity.id(),
                    archetype_entity.table_row(),
                )
            };

            accum = func(accum, item);
        }
        accum
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> Iterator for QueryIter<'w, 's, D, F> {
    type Item = D::Item<'w>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // `tables` and `archetypes` belong to the same world that the cursor was initialized for.
        // `query_state` is the state that was passed to `QueryIterationCursor::init`.
        unsafe {
            self.cursor
                .next(self.tables, self.archetypes, self.query_state)
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let max_size = self.cursor.max_remaining(self.tables, self.archetypes);
        let archetype_query = F::IS_ARCHETYPAL;
        let min_size = if archetype_query { max_size } else { 0 };
        (min_size, Some(max_size))
    }

    #[inline]
    fn fold<B, Func>(mut self, init: B, mut func: Func) -> B
    where
        Func: FnMut(B, Self::Item) -> B,
    {
        let mut accum = init;
        // Empty any remaining uniterated values from the current table/archetype
        while self.cursor.current_row != self.cursor.current_len {
            let Some(item) = self.next() else { break };
            accum = func(accum, item);
        }
        for id in self.cursor.storage_id_iter.clone() {
            if D::IS_DENSE && F::IS_DENSE {
                // SAFETY: Matched table IDs are guaranteed to still exist.
                let table = unsafe { self.tables.get(id.table_id).debug_checked_unwrap() };
                accum =
                    // SAFETY: 
                    // - The fetched table matches both D and F
                    // - The provided range is equivalent to [0, table.entity_count)
                    // - The if block ensures that D::IS_DENSE and F::IS_DENSE are both true
                    unsafe { self.fold_over_table_range(accum, &mut func, table, 0..table.entity_count()) };
            } else {
                let archetype =
                    // SAFETY: Matched archetype IDs are guaranteed to still exist.
                    unsafe { self.archetypes.get(id.archetype_id).debug_checked_unwrap() };
                accum =
                    // SAFETY:
                    // - The fetched archetype matches both D and F
                    // - The provided range is equivalent to [0, archetype.len)
                    // - The if block ensures that ether D::IS_DENSE or F::IS_DENSE are false
                    unsafe { self.fold_over_archetype_range(accum, &mut func, archetype, 0..archetype.len()) };
            }
        }
        accum
    }
}

// This is correct as [`QueryIter`] always returns `None` once exhausted.
impl<'w, 's, D: QueryData, F: QueryFilter> FusedIterator for QueryIter<'w, 's, D, F> {}

/// An [`Iterator`] over the query items generated from an iterator of [`Entity`]s.
///
/// Items are returned in the order of the provided iterator.
/// Entities that don't match the query are skipped.
///
/// This struct is created by the [`Query::iter_many`](crate::system::Query::iter_many) and [`Query::iter_many_mut`](crate::system::Query::iter_many_mut) methods.
pub struct QueryManyIter<'w, 's, D: QueryData, F: QueryFilter, I: Iterator>
where
    I::Item: Borrow<Entity>,
{
    entity_iter: I,
    entities: &'w Entities,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    fetch: D::Fetch<'w>,
    filter: F::Fetch<'w>,
    query_state: &'s QueryState<D, F>,
}

impl<'w, 's, D: QueryData, F: QueryFilter, I: Iterator> QueryManyIter<'w, 's, D, F, I>
where
    I::Item: Borrow<Entity>,
{
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    pub(crate) unsafe fn new<EntityList: IntoIterator<IntoIter = I>>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        entity_list: EntityList,
        last_run: Tick,
        this_run: Tick,
    ) -> QueryManyIter<'w, 's, D, F, I> {
        let fetch = D::init_fetch(world, &query_state.fetch_state, last_run, this_run);
        let filter = F::init_fetch(world, &query_state.filter_state, last_run, this_run);
        QueryManyIter {
            query_state,
            entities: world.entities(),
            archetypes: world.archetypes(),
            // SAFETY: We only access table data that has been registered in `query_state`.
            // This means `world` has permission to access the data we use.
            tables: &world.storages().tables,
            fetch,
            filter,
            entity_iter: entity_list.into_iter(),
        }
    }

    /// Safety:
    /// The lifetime here is not restrictive enough for Fetch with &mut access,
    /// as calling `fetch_next_aliased_unchecked` multiple times can produce multiple
    /// references to the same component, leading to unique reference aliasing.
    ///
    /// It is always safe for shared access.
    #[inline(always)]
    unsafe fn fetch_next_aliased_unchecked(&mut self) -> Option<D::Item<'w>> {
        for entity in self.entity_iter.by_ref() {
            let entity = *entity.borrow();
            let Some(location) = self.entities.get(entity) else {
                continue;
            };

            if !self
                .query_state
                .matched_archetypes
                .contains(location.archetype_id.index())
            {
                continue;
            }

            let archetype = self
                .archetypes
                .get(location.archetype_id)
                .debug_checked_unwrap();
            let table = self.tables.get(location.table_id).debug_checked_unwrap();

            // SAFETY: `archetype` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            unsafe {
                D::set_archetype(
                    &mut self.fetch,
                    &self.query_state.fetch_state,
                    archetype,
                    table,
                );
            }
            // SAFETY: `table` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            unsafe {
                F::set_archetype(
                    &mut self.filter,
                    &self.query_state.filter_state,
                    archetype,
                    table,
                );
            }

            // SAFETY: set_archetype was called prior.
            // `location.archetype_row` is an archetype index row in range of the current archetype, because if it was not, the match above would have `continue`d
            if unsafe { F::filter_fetch(&mut self.filter, entity, location.table_row) } {
                // SAFETY:
                // - set_archetype was called prior, `location.archetype_row` is an archetype index in range of the current archetype
                // - fetch is only called once for each entity.
                return Some(unsafe { D::fetch(&mut self.fetch, entity, location.table_row) });
            }
        }
        None
    }

    /// Get next result from the query
    #[inline(always)]
    pub fn fetch_next(&mut self) -> Option<D::Item<'_>> {
        // SAFETY: we are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe { self.fetch_next_aliased_unchecked().map(D::shrink) }
    }
}

impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, I: Iterator> Iterator
    for QueryManyIter<'w, 's, D, F, I>
where
    I::Item: Borrow<Entity>,
{
    type Item = D::Item<'w>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: It is safe to alias for ReadOnlyWorldQuery.
        unsafe { self.fetch_next_aliased_unchecked() }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (_, max_size) = self.entity_iter.size_hint();
        (0, max_size)
    }
}

// This is correct as [`QueryManyIter`] always returns `None` once exhausted.
impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, I: Iterator> FusedIterator
    for QueryManyIter<'w, 's, D, F, I>
where
    I::Item: Borrow<Entity>,
{
}

/// An iterator over `K`-sized combinations of query items without repetition.
///
/// A combination is an arrangement of a collection of items where order does not matter.
///
/// `K` is the number of items that make up each subset, and the number of items returned by the iterator.
/// `N` is the number of total entities output by the query.
///
/// For example, given the list [1, 2, 3, 4], where `K` is 2, the combinations without repeats are
/// [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4].
/// And in this case, `N` would be defined as 4 since the size of the input list is 4.
///
/// The number of combinations depend on how `K` relates to the number of entities matching the [`Query`]:
/// - if `K = N`, only one combination exists,
/// - if `K < N`, there are <sub>N</sub>C<sub>K</sub> combinations (see the [performance section] of `Query`),
/// - if `K > N`, there are no combinations.
///
/// The output combination is not guaranteed to have any order of iteration.
///
/// # Usage
///
/// This type is returned by calling [`Query::iter_combinations`] or [`Query::iter_combinations_mut`].
///
/// It implements [`Iterator`] only if it iterates over read-only query items ([learn more]).
///
/// In the case of mutable query items, it can be iterated by calling [`fetch_next`] in a `while let` loop.
///
/// # Examples
///
/// The following example shows how to traverse the iterator when the query items are read-only.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// fn some_system(query: Query<&ComponentA>) {
///     for [a1, a2] in query.iter_combinations() {
///         // ...
///     }
/// }
/// ```
///
/// The following example shows how `fetch_next` should be called with a `while let` loop to traverse the iterator when the query items are mutable.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)]
/// # struct ComponentA;
/// #
/// fn some_system(mut query: Query<&mut ComponentA>) {
///     let mut combinations = query.iter_combinations_mut();
///     while let Some([a1, a2]) = combinations.fetch_next() {
///         // ...
///     }
/// }
/// ```
///
/// [`fetch_next`]: Self::fetch_next
/// [learn more]: Self#impl-Iterator
/// [performance section]: crate::system::Query#performance
/// [`Query`]: crate::system::Query
/// [`Query::iter_combinations`]: crate::system::Query::iter_combinations
/// [`Query::iter_combinations_mut`]: crate::system::Query::iter_combinations_mut
pub struct QueryCombinationIter<'w, 's, D: QueryData, F: QueryFilter, const K: usize> {
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<D, F>,
    cursors: [QueryIterationCursor<'w, 's, D, F>; K],
}

impl<'w, 's, D: QueryData, F: QueryFilter, const K: usize> QueryCombinationIter<'w, 's, D, F, K> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        // Initialize array with cursors.
        // There is no FromIterator on arrays, so instead initialize it manually with MaybeUninit

        let mut array: MaybeUninit<[QueryIterationCursor<'w, 's, D, F>; K]> = MaybeUninit::uninit();
        let ptr = array
            .as_mut_ptr()
            .cast::<QueryIterationCursor<'w, 's, D, F>>();
        if K != 0 {
            ptr.write(QueryIterationCursor::init(
                world,
                query_state,
                last_run,
                this_run,
            ));
        }
        for slot in (1..K).map(|offset| ptr.add(offset)) {
            slot.write(QueryIterationCursor::init_empty(
                world,
                query_state,
                last_run,
                this_run,
            ));
        }

        QueryCombinationIter {
            query_state,
            // SAFETY: We only access table data that has been registered in `query_state`.
            tables: unsafe { &world.storages().tables },
            archetypes: world.archetypes(),
            cursors: array.assume_init(),
        }
    }

    /// Safety:
    /// The lifetime here is not restrictive enough for Fetch with &mut access,
    /// as calling `fetch_next_aliased_unchecked` multiple times can produce multiple
    /// references to the same component, leading to unique reference aliasing.
    ///.
    /// It is always safe for shared access.
    unsafe fn fetch_next_aliased_unchecked(&mut self) -> Option<[D::Item<'w>; K]> {
        if K == 0 {
            return None;
        }

        // PERF: can speed up the following code using `cursor.remaining()` instead of `next_item.is_none()`
        // when D::IS_ARCHETYPAL && F::IS_ARCHETYPAL
        //
        // let `i` be the index of `c`, the last cursor in `self.cursors` that
        // returns `K-i` or more elements.
        // Make cursor in index `j` for all `j` in `[i, K)` a copy of `c` advanced `j-i+1` times.
        // If no such `c` exists, return `None`
        'outer: for i in (0..K).rev() {
            match self.cursors[i].next(self.tables, self.archetypes, self.query_state) {
                Some(_) => {
                    for j in (i + 1)..K {
                        self.cursors[j] = self.cursors[j - 1].clone();
                        match self.cursors[j].next(self.tables, self.archetypes, self.query_state) {
                            Some(_) => {}
                            None if i > 0 => continue 'outer,
                            None => return None,
                        }
                    }
                    break;
                }
                None if i > 0 => continue,
                None => return None,
            }
        }

        let mut values = MaybeUninit::<[D::Item<'w>; K]>::uninit();

        let ptr = values.as_mut_ptr().cast::<D::Item<'w>>();
        for (offset, cursor) in self.cursors.iter_mut().enumerate() {
            ptr.add(offset).write(cursor.peek_last().unwrap());
        }

        Some(values.assume_init())
    }

    /// Get next combination of queried components
    #[inline]
    pub fn fetch_next(&mut self) -> Option<[D::Item<'_>; K]> {
        // SAFETY: we are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe {
            self.fetch_next_aliased_unchecked()
                .map(|array| array.map(D::shrink))
        }
    }
}

// Iterator type is intentionally implemented only for read-only access.
// Doing so for mutable references would be unsound, because calling `next`
// multiple times would allow multiple owned references to the same data to exist.
impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, const K: usize> Iterator
    for QueryCombinationIter<'w, 's, D, F, K>
{
    type Item = [D::Item<'w>; K];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Safety: it is safe to alias for ReadOnlyWorldQuery
        unsafe { QueryCombinationIter::fetch_next_aliased_unchecked(self) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // binomial coefficient: (n ; k) = n! / k!(n-k)! = (n*n-1*...*n-k+1) / k!
        // See https://en.wikipedia.org/wiki/Binomial_coefficient
        // See https://blog.plover.com/math/choose.html for implementation
        // It was chosen to reduce overflow potential.
        fn choose(n: usize, k: usize) -> Option<usize> {
            if k > n || n == 0 {
                return Some(0);
            }
            let k = k.min(n - k);
            let ks = 1..=k;
            let ns = (n - k + 1..=n).rev();
            ks.zip(ns)
                .try_fold(1_usize, |acc, (k, n)| Some(acc.checked_mul(n)? / k))
        }
        // sum_i=0..k choose(cursors[i].remaining, k-i)
        let max_combinations = self
            .cursors
            .iter()
            .enumerate()
            .try_fold(0, |acc, (i, cursor)| {
                let n = cursor.max_remaining(self.tables, self.archetypes);
                Some(acc + choose(n, K - i)?)
            });

        let archetype_query = F::IS_ARCHETYPAL;
        let known_max = max_combinations.unwrap_or(usize::MAX);
        let min_combinations = if archetype_query { known_max } else { 0 };
        (min_combinations, max_combinations)
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> ExactSizeIterator for QueryIter<'w, 's, D, F>
where
    F: ArchetypeFilter,
{
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

// This is correct as [`QueryCombinationIter`] always returns `None` once exhausted.
impl<'w, 's, D: ReadOnlyQueryData, F: QueryFilter, const K: usize> FusedIterator
    for QueryCombinationIter<'w, 's, D, F, K>
{
}

struct QueryIterationCursor<'w, 's, D: QueryData, F: QueryFilter> {
    storage_id_iter: std::slice::Iter<'s, StorageId>,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    fetch: D::Fetch<'w>,
    filter: F::Fetch<'w>,
    // length of the table or length of the archetype, depending on whether both `D`'s and `F`'s fetches are dense
    current_len: usize,
    // either table row or archetype index, depending on whether both `D`'s and `F`'s fetches are dense
    current_row: usize,
}

impl<D: QueryData, F: QueryFilter> Clone for QueryIterationCursor<'_, '_, D, F> {
    fn clone(&self) -> Self {
        Self {
            storage_id_iter: self.storage_id_iter.clone(),
            table_entities: self.table_entities,
            archetype_entities: self.archetype_entities,
            fetch: self.fetch.clone(),
            filter: self.filter.clone(),
            current_len: self.current_len,
            current_row: self.current_row,
        }
    }
}

impl<'w, 's, D: QueryData, F: QueryFilter> QueryIterationCursor<'w, 's, D, F> {
    const IS_DENSE: bool = D::IS_DENSE && F::IS_DENSE;

    unsafe fn init_empty(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        QueryIterationCursor {
            storage_id_iter: [].iter(),
            ..Self::init(world, query_state, last_run, this_run)
        }
    }

    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    unsafe fn init(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        let fetch = D::init_fetch(world, &query_state.fetch_state, last_run, this_run);
        let filter = F::init_fetch(world, &query_state.filter_state, last_run, this_run);
        QueryIterationCursor {
            fetch,
            filter,
            table_entities: &[],
            archetype_entities: &[],
            storage_id_iter: query_state.matched_storage_ids.iter(),
            current_len: 0,
            current_row: 0,
        }
    }

    /// retrieve item returned from most recent `next` call again.
    #[inline]
    unsafe fn peek_last(&mut self) -> Option<D::Item<'w>> {
        if self.current_row > 0 {
            let index = self.current_row - 1;
            if Self::IS_DENSE {
                let entity = self.table_entities.get_unchecked(index);
                Some(D::fetch(
                    &mut self.fetch,
                    *entity,
                    TableRow::from_usize(index),
                ))
            } else {
                let archetype_entity = self.archetype_entities.get_unchecked(index);
                Some(D::fetch(
                    &mut self.fetch,
                    archetype_entity.id(),
                    archetype_entity.table_row(),
                ))
            }
        } else {
            None
        }
    }

    /// How many values will this cursor return at most?
    ///
    /// Note that if `F::IS_ARCHETYPAL`, the return value
    /// will be **the exact count of remaining values**.
    fn max_remaining(&self, tables: &'w Tables, archetypes: &'w Archetypes) -> usize {
        let ids = self.storage_id_iter.clone();
        let remaining_matched: usize = if Self::IS_DENSE {
            // SAFETY: The if check ensures that storage_id_iter stores TableIds
            unsafe { ids.map(|id| tables[id.table_id].entity_count()).sum() }
        } else {
            // SAFETY: The if check ensures that storage_id_iter stores ArchetypeIds
            unsafe { ids.map(|id| archetypes[id.archetype_id].len()).sum() }
        };
        remaining_matched + self.current_len - self.current_row
    }

    // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
    // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter, QueryState::par_for_each_unchecked_manual
    /// # Safety
    /// `tables` and `archetypes` must belong to the same world that the [`QueryIterationCursor`]
    /// was initialized for.
    /// `query_state` must be the same [`QueryState`] that was passed to `init` or `init_empty`.
    #[inline(always)]
    unsafe fn next(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s QueryState<D, F>,
    ) -> Option<D::Item<'w>> {
        if Self::IS_DENSE {
            loop {
                // we are on the beginning of the query, or finished processing a table, so skip to the next
                if self.current_row == self.current_len {
                    let table_id = self.storage_id_iter.next()?.table_id;
                    let table = tables.get(table_id).debug_checked_unwrap();
                    // SAFETY: `table` is from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    unsafe {
                        D::set_table(&mut self.fetch, &query_state.fetch_state, table);
                        F::set_table(&mut self.filter, &query_state.filter_state, table);
                    }
                    self.table_entities = table.entities();
                    self.current_len = table.entity_count();
                    self.current_row = 0;
                    continue;
                }

                // SAFETY: set_table was called prior.
                // `current_row` is a table row in range of the current table, because if it was not, then the above would have been executed.
                let entity = unsafe { self.table_entities.get_unchecked(self.current_row) };
                let row = TableRow::from_usize(self.current_row);
                if !F::filter_fetch(&mut self.filter, *entity, row) {
                    self.current_row += 1;
                    continue;
                }

                // SAFETY:
                // - set_table was called prior.
                // - `current_row` must be a table row in range of the current table,
                //   because if it was not, then the above would have been executed.
                // - fetch is only called once for each `entity`.
                let item = unsafe { D::fetch(&mut self.fetch, *entity, row) };

                self.current_row += 1;
                return Some(item);
            }
        } else {
            loop {
                if self.current_row == self.current_len {
                    let archetype_id = self.storage_id_iter.next()?.archetype_id;
                    let archetype = archetypes.get(archetype_id).debug_checked_unwrap();
                    let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                    // SAFETY: `archetype` and `tables` are from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    unsafe {
                        D::set_archetype(
                            &mut self.fetch,
                            &query_state.fetch_state,
                            archetype,
                            table,
                        );
                        F::set_archetype(
                            &mut self.filter,
                            &query_state.filter_state,
                            archetype,
                            table,
                        );
                    }
                    self.archetype_entities = archetype.entities();
                    self.current_len = archetype.len();
                    self.current_row = 0;
                    continue;
                }

                // SAFETY: set_archetype was called prior.
                // `current_row` is an archetype index row in range of the current archetype, because if it was not, then the if above would have been executed.
                let archetype_entity =
                    unsafe { self.archetype_entities.get_unchecked(self.current_row) };
                if !F::filter_fetch(
                    &mut self.filter,
                    archetype_entity.id(),
                    archetype_entity.table_row(),
                ) {
                    self.current_row += 1;
                    continue;
                }

                // SAFETY:
                // - set_archetype was called prior.
                // - `current_row` must be an archetype index row in range of the current archetype,
                //   because if it was not, then the if above would have been executed.
                // - fetch is only called once for each `archetype_entity`.
                let item = unsafe {
                    D::fetch(
                        &mut self.fetch,
                        archetype_entity.id(),
                        archetype_entity.table_row(),
                    )
                };
                self.current_row += 1;
                return Some(item);
            }
        }
    }
}
