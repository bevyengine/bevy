use crate::{
    archetype::{ArchetypeId, Archetypes},
    entity::{Entities, Entity},
    prelude::World,
    query::{ArchetypeFilter, QueryState, WorldQuery},
    storage::{TableId, Tables},
};
use std::{borrow::Borrow, iter::FusedIterator, marker::PhantomData, mem::MaybeUninit};

use super::{QueryFetch, QueryItem, ReadOnlyWorldQuery};

/// An [`Iterator`] over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::iter`](crate::system::Query::iter) and
/// [`Query::iter_mut`](crate::system::Query::iter_mut) methods.
pub struct QueryIter<'w, 's, Q: WorldQuery, F: WorldQuery> {
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    cursor: QueryIterationCursor<'w, 's, Q, F>,
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> QueryIter<'w, 's, Q, F> {
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `query_state.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`](crate::world::WorldId) is unsound.
    pub(crate) unsafe fn new(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        QueryIter {
            query_state,
            tables: &world.storages().tables,
            archetypes: &world.archetypes,
            cursor: QueryIterationCursor::init(world, query_state, last_change_tick, change_tick),
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> Iterator for QueryIter<'w, 's, Q, F> {
    type Item = QueryItem<'w, Q>;

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
        let max_size = self
            .query_state
            .matched_archetype_ids
            .iter()
            .map(|id| self.archetypes[*id].len())
            .sum();

        let archetype_query = Q::IS_ARCHETYPAL && F::IS_ARCHETYPAL;
        let min_size = if archetype_query { max_size } else { 0 };
        (min_size, Some(max_size))
    }
}

// This is correct as [`QueryIter`] always returns `None` once exhausted.
impl<'w, 's, Q: WorldQuery, F: WorldQuery> FusedIterator for QueryIter<'w, 's, Q, F> {}

/// An [`Iterator`] over [`Query`](crate::system::Query) results of a list of [`Entity`]s.
///
/// This struct is created by the [`Query::iter_many`](crate::system::Query::iter_many) and [`Query::iter_many_mut`](crate::system::Query::iter_many_mut) methods.
pub struct QueryManyIter<'w, 's, Q: WorldQuery, F: WorldQuery, I: Iterator>
where
    I::Item: Borrow<Entity>,
{
    entity_iter: I,
    entities: &'w Entities,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    fetch: QueryFetch<'w, Q>,
    filter: QueryFetch<'w, F>,
    query_state: &'s QueryState<Q, F>,
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery, I: Iterator> QueryManyIter<'w, 's, Q, F, I>
where
    I::Item: Borrow<Entity>,
{
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `query_state.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`](crate::world::WorldId) is unsound.
    pub(crate) unsafe fn new<EntityList: IntoIterator<IntoIter = I>>(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        entity_list: EntityList,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryManyIter<'w, 's, Q, F, I> {
        let fetch = Q::init_fetch(
            world,
            &query_state.fetch_state,
            last_change_tick,
            change_tick,
        );
        let filter = F::init_fetch(
            world,
            &query_state.filter_state,
            last_change_tick,
            change_tick,
        );
        QueryManyIter {
            query_state,
            entities: &world.entities,
            archetypes: &world.archetypes,
            tables: &world.storages.tables,
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
    unsafe fn fetch_next_aliased_unchecked(&mut self) -> Option<QueryItem<'w, Q>> {
        for entity in self.entity_iter.by_ref() {
            let location = match self.entities.get(*entity.borrow()) {
                Some(location) => location,
                None => continue,
            };

            if !self
                .query_state
                .matched_archetypes
                .contains(location.archetype_id.index())
            {
                continue;
            }

            let archetype = &self.archetypes[location.archetype_id];

            // SAFETY: `archetype` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            Q::set_archetype(
                &mut self.fetch,
                &self.query_state.fetch_state,
                archetype,
                self.tables,
            );
            // SAFETY: `table` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            F::set_archetype(
                &mut self.filter,
                &self.query_state.filter_state,
                archetype,
                self.tables,
            );
            // SAFETY: set_archetype was called prior.
            // `location.index` is an archetype index row in range of the current archetype, because if it was not, the match above would have `continue`d
            if F::archetype_filter_fetch(&mut self.filter, location.index) {
                // SAFETY: set_archetype was called prior, `location.index` is an archetype index in range of the current archetype
                return Some(Q::archetype_fetch(&mut self.fetch, location.index));
            }
        }
        None
    }

    /// Get next result from the query
    #[inline(always)]
    pub fn fetch_next(&mut self) -> Option<QueryItem<'_, Q>> {
        // SAFETY: we are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe { self.fetch_next_aliased_unchecked().map(Q::shrink) }
    }
}

impl<'w, 's, Q: ReadOnlyWorldQuery, F: ReadOnlyWorldQuery, I: Iterator> Iterator
    for QueryManyIter<'w, 's, Q, F, I>
where
    I::Item: Borrow<Entity>,
{
    type Item = QueryItem<'w, Q>;

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
impl<'w, 's, Q: ReadOnlyWorldQuery, F: ReadOnlyWorldQuery, I: Iterator> FusedIterator
    for QueryManyIter<'w, 's, Q, F, I>
where
    I::Item: Borrow<Entity>,
{
}

/// An iterator over `K`-sized combinations of query items without repetition.
///
/// In this context, a combination is an unordered subset of `K` elements.
/// The number of combinations depend on how `K` relates to the number of entities matching the [`Query`] (called `N`):
/// - if `K = N`, only one combination exists,
/// - if `K < N`, there are <sub>N</sub>C<sub>K</sub> combinations (see the [performance section] of `Query`),
/// - if `K > N`, there are no combinations.
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
pub struct QueryCombinationIter<'w, 's, Q: WorldQuery, F: WorldQuery, const K: usize> {
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    cursors: [QueryIterationCursor<'w, 's, Q, F>; K],
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery, const K: usize> QueryCombinationIter<'w, 's, Q, F, K> {
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `query_state.world_id`. Calling this on a
    /// `world` with a mismatched [`WorldId`](crate::world::WorldId) is unsound.
    pub(crate) unsafe fn new(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        // Initialize array with cursors.
        // There is no FromIterator on arrays, so instead initialize it manually with MaybeUninit

        let mut array: MaybeUninit<[QueryIterationCursor<'w, 's, Q, F>; K]> = MaybeUninit::uninit();
        let ptr = array
            .as_mut_ptr()
            .cast::<QueryIterationCursor<'w, 's, Q, F>>();
        if K != 0 {
            ptr.write(QueryIterationCursor::init(
                world,
                query_state,
                last_change_tick,
                change_tick,
            ));
        }
        for slot in (1..K).map(|offset| ptr.add(offset)) {
            slot.write(QueryIterationCursor::init_empty(
                world,
                query_state,
                last_change_tick,
                change_tick,
            ));
        }

        QueryCombinationIter {
            query_state,
            tables: &world.storages().tables,
            archetypes: &world.archetypes,
            cursors: array.assume_init(),
        }
    }

    /// Safety:
    /// The lifetime here is not restrictive enough for Fetch with &mut access,
    /// as calling `fetch_next_aliased_unchecked` multiple times can produce multiple
    /// references to the same component, leading to unique reference aliasing.
    ///.
    /// It is always safe for shared access.
    unsafe fn fetch_next_aliased_unchecked(&mut self) -> Option<[QueryItem<'w, Q>; K]>
    where
        QueryFetch<'w, Q>: Clone,
        QueryFetch<'w, F>: Clone,
    {
        if K == 0 {
            return None;
        }

        // first, iterate from last to first until next item is found
        'outer: for i in (0..K).rev() {
            match self.cursors[i].next(self.tables, self.archetypes, self.query_state) {
                Some(_) => {
                    // walk forward up to last element, propagating cursor state forward
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

        let mut values = MaybeUninit::<[QueryItem<'w, Q>; K]>::uninit();

        let ptr = values.as_mut_ptr().cast::<QueryItem<'w, Q>>();
        for (offset, cursor) in self.cursors.iter_mut().enumerate() {
            ptr.add(offset).write(cursor.peek_last().unwrap());
        }

        Some(values.assume_init())
    }

    /// Get next combination of queried components
    #[inline]
    pub fn fetch_next(&mut self) -> Option<[QueryItem<'_, Q>; K]>
    where
        for<'a> QueryFetch<'a, Q>: Clone,
        for<'a> QueryFetch<'a, F>: Clone,
    {
        // SAFETY: we are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe {
            self.fetch_next_aliased_unchecked()
                .map(|array| array.map(Q::shrink))
        }
    }
}

// Iterator type is intentionally implemented only for read-only access.
// Doing so for mutable references would be unsound, because  calling `next`
// multiple times would allow multiple owned references to the same data to exist.
impl<'w, 's, Q: ReadOnlyWorldQuery, F: ReadOnlyWorldQuery, const K: usize> Iterator
    for QueryCombinationIter<'w, 's, Q, F, K>
where
    QueryFetch<'w, Q>: Clone,
    QueryFetch<'w, F>: Clone,
{
    type Item = [QueryItem<'w, Q>; K];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Safety: it is safe to alias for ReadOnlyWorldQuery
        unsafe { QueryCombinationIter::fetch_next_aliased_unchecked(self) }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if K == 0 {
            return (0, Some(0));
        }

        let max_size: usize = self
            .query_state
            .matched_archetype_ids
            .iter()
            .map(|id| self.archetypes[*id].len())
            .sum();

        if max_size < K {
            return (0, Some(0));
        }
        if max_size == K {
            return (1, Some(1));
        }

        // binomial coefficient: (n ; k) = n! / k!(n-k)! = (n*n-1*...*n-k+1) / k!
        // See https://en.wikipedia.org/wiki/Binomial_coefficient
        // See https://blog.plover.com/math/choose.html for implementation
        // It was chosen to reduce overflow potential.
        fn choose(n: usize, k: usize) -> Option<usize> {
            let ks = 1..=k;
            let ns = (n - k + 1..=n).rev();
            ks.zip(ns)
                .try_fold(1_usize, |acc, (k, n)| Some(acc.checked_mul(n)? / k))
        }
        let smallest = K.min(max_size - K);
        let max_combinations = choose(max_size, smallest);

        let archetype_query = F::IS_ARCHETYPAL && Q::IS_ARCHETYPAL;
        let known_max = max_combinations.unwrap_or(usize::MAX);
        let min_combinations = if archetype_query { known_max } else { 0 };
        (min_combinations, max_combinations)
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> ExactSizeIterator for QueryIter<'w, 's, Q, F>
where
    F: ArchetypeFilter,
{
    fn len(&self) -> usize {
        self.query_state
            .matched_archetype_ids
            .iter()
            .map(|id| self.archetypes[*id].len())
            .sum()
    }
}

impl<'w, 's, Q: ReadOnlyWorldQuery, F: ReadOnlyWorldQuery + ArchetypeFilter, const K: usize>
    ExactSizeIterator for QueryCombinationIter<'w, 's, Q, F, K>
where
    QueryFetch<'w, Q>: Clone,
    QueryFetch<'w, F>: Clone,
{
    /// Returns the exact length of the iterator.
    ///
    /// **NOTE**: When the iterator length overflows `usize`, this will
    /// return `usize::MAX`.
    fn len(&self) -> usize {
        self.size_hint().0
    }
}

// This is correct as [`QueryCombinationIter`] always returns `None` once exhausted.
impl<'w, 's, Q: ReadOnlyWorldQuery, F: ReadOnlyWorldQuery, const K: usize> FusedIterator
    for QueryCombinationIter<'w, 's, Q, F, K>
where
    QueryFetch<'w, Q>: Clone,
    QueryFetch<'w, F>: Clone,
{
}

struct QueryIterationCursor<'w, 's, Q: WorldQuery, F: WorldQuery> {
    table_id_iter: std::slice::Iter<'s, TableId>,
    archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
    fetch: QueryFetch<'w, Q>,
    filter: QueryFetch<'w, F>,
    // length of the table table or length of the archetype, depending on whether both `Q`'s and `F`'s fetches are dense
    current_len: usize,
    // either table row or archetype index, depending on whether both `Q`'s and `F`'s fetches are dense
    current_index: usize,
    phantom: PhantomData<Q>,
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> Clone for QueryIterationCursor<'w, 's, Q, F>
where
    QueryFetch<'w, Q>: Clone,
    QueryFetch<'w, F>: Clone,
{
    fn clone(&self) -> Self {
        Self {
            table_id_iter: self.table_id_iter.clone(),
            archetype_id_iter: self.archetype_id_iter.clone(),
            fetch: self.fetch.clone(),
            filter: self.filter.clone(),
            current_len: self.current_len,
            current_index: self.current_index,
            phantom: PhantomData,
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> QueryIterationCursor<'w, 's, Q, F> {
    const IS_DENSE: bool = Q::IS_DENSE && F::IS_DENSE;

    unsafe fn init_empty(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        QueryIterationCursor {
            table_id_iter: [].iter(),
            archetype_id_iter: [].iter(),
            ..Self::init(world, query_state, last_change_tick, change_tick)
        }
    }

    unsafe fn init(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        let fetch = Q::init_fetch(
            world,
            &query_state.fetch_state,
            last_change_tick,
            change_tick,
        );
        let filter = F::init_fetch(
            world,
            &query_state.filter_state,
            last_change_tick,
            change_tick,
        );
        QueryIterationCursor {
            fetch,
            filter,
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            current_len: 0,
            current_index: 0,
            phantom: PhantomData,
        }
    }

    /// retrieve item returned from most recent `next` call again.
    #[inline]
    unsafe fn peek_last(&mut self) -> Option<QueryItem<'w, Q>> {
        if self.current_index > 0 {
            if Self::IS_DENSE {
                Some(Q::table_fetch(&mut self.fetch, self.current_index - 1))
            } else {
                Some(Q::archetype_fetch(&mut self.fetch, self.current_index - 1))
            }
        } else {
            None
        }
    }

    // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
    // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
    /// # Safety
    /// `tables` and `archetypes` must belong to the same world that the [`QueryIterationCursor`]
    /// was initialized for.
    /// `query_state` must be the same [`QueryState`] that was passed to `init` or `init_empty`.
    #[inline(always)]
    unsafe fn next(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s QueryState<Q, F>,
    ) -> Option<QueryItem<'w, Q>> {
        if Self::IS_DENSE {
            loop {
                // we are on the beginning of the query, or finished processing a table, so skip to the next
                if self.current_index == self.current_len {
                    let table_id = self.table_id_iter.next()?;
                    let table = &tables[*table_id];
                    // SAFETY: `table` is from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    Q::set_table(&mut self.fetch, &query_state.fetch_state, table);
                    F::set_table(&mut self.filter, &query_state.filter_state, table);
                    self.current_len = table.len();
                    self.current_index = 0;
                    continue;
                }

                // SAFETY: set_table was called prior.
                // `current_index` is a table row in range of the current table, because if it was not, then the if above would have been executed.
                if !F::table_filter_fetch(&mut self.filter, self.current_index) {
                    self.current_index += 1;
                    continue;
                }

                // SAFETY: set_table was called prior.
                // `current_index` is a table row in range of the current table, because if it was not, then the if above would have been executed.
                let item = Q::table_fetch(&mut self.fetch, self.current_index);

                self.current_index += 1;
                return Some(item);
            }
        } else {
            loop {
                if self.current_index == self.current_len {
                    let archetype_id = self.archetype_id_iter.next()?;
                    let archetype = &archetypes[*archetype_id];
                    // SAFETY: `archetype` and `tables` are from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    Q::set_archetype(&mut self.fetch, &query_state.fetch_state, archetype, tables);
                    F::set_archetype(
                        &mut self.filter,
                        &query_state.filter_state,
                        archetype,
                        tables,
                    );
                    self.current_len = archetype.len();
                    self.current_index = 0;
                    continue;
                }

                // SAFETY: set_archetype was called prior.
                // `current_index` is an archetype index row in range of the current archetype, because if it was not, then the if above would have been executed.
                if !F::archetype_filter_fetch(&mut self.filter, self.current_index) {
                    self.current_index += 1;
                    continue;
                }

                // SAFETY: set_archetype was called prior, `current_index` is an archetype index in range of the current archetype
                // `current_index` is an archetype index row in range of the current archetype, because if it was not, then the if above would have been executed.
                let item = Q::archetype_fetch(&mut self.fetch, self.current_index);
                self.current_index += 1;
                return Some(item);
            }
        }
    }
}
