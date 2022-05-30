use crate::{
    archetype::{ArchetypeId, Archetypes},
    query::{Fetch, QueryState, WorldQuery},
    storage::{TableId, Tables},
    world::World,
};
use std::{marker::PhantomData, mem::MaybeUninit};

use super::{QueryFetch, QueryItem, ReadOnlyFetch};

/// An [`Iterator`] over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::iter`](crate::system::Query::iter) and
/// [`Query::iter_mut`](crate::system::Query::iter_mut) methods.
pub struct QueryIter<'w, 's, Q: WorldQuery, QF: Fetch<'w, State = Q::State>, F: WorldQuery> {
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    cursor: QueryIterationCursor<'w, 's, Q, QF, F>,
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> QueryIter<'w, 's, Q, QF, F>
where
    QF: Fetch<'w, State = Q::State>,
{
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

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> Iterator for QueryIter<'w, 's, Q, QF, F>
where
    QF: Fetch<'w, State = Q::State>,
{
    type Item = QF::Item;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
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

        let archetype_query = F::Fetch::IS_ARCHETYPAL && QF::IS_ARCHETYPAL;
        let min_size = if archetype_query { max_size } else { 0 };
        (min_size, Some(max_size))
    }
}

pub struct QueryCombinationIter<'w, 's, Q: WorldQuery, F: WorldQuery, const K: usize> {
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    cursors: [QueryIterationCursor<'w, 's, Q, QueryFetch<'w, Q>, F>; K],
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

        let mut array: MaybeUninit<[QueryIterationCursor<'w, 's, Q, QueryFetch<'w, Q>, F>; K]> =
            MaybeUninit::uninit();
        let ptr = array
            .as_mut_ptr()
            .cast::<QueryIterationCursor<'w, 's, Q, QueryFetch<'w, Q>, F>>();
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
            ptr.add(offset).write(cursor.peek_last().unwrap())
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
        // safety: we are limiting the returned reference to self,
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
impl<'w, 's, Q: WorldQuery, F: WorldQuery, const K: usize> Iterator
    for QueryCombinationIter<'w, 's, Q, F, K>
where
    QueryFetch<'w, Q>: Clone + ReadOnlyFetch,
    QueryFetch<'w, F>: Clone + ReadOnlyFetch,
{
    type Item = [QueryItem<'w, Q>; K];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Safety: it is safe to alias for ReadOnlyFetch
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

        // n! / k!(n-k)! = (n*n-1*...*n-k+1) / k!
        let max_combinations = (0..K)
            .try_fold(1usize, |n, i| n.checked_mul(max_size - i))
            .map(|n| {
                let k_factorial: usize = (1..=K).product();
                n / k_factorial
            });

        let archetype_query = F::Fetch::IS_ARCHETYPAL && Q::Fetch::IS_ARCHETYPAL;
        let min_combinations = if archetype_query { max_size } else { 0 };
        (min_combinations, max_combinations)
    }
}

// NOTE: We can cheaply implement this for unfiltered Queries because we have:
// (1) pre-computed archetype matches
// (2) each archetype pre-computes length
// (3) there are no per-entity filters
// TODO: add an ArchetypeOnlyFilter that enables us to implement this for filters like With<T>.
// This would need to be added to all types that implement Filter with Filter::IS_ARCHETYPAL = true
impl<'w, 's, Q: WorldQuery, QF> ExactSizeIterator for QueryIter<'w, 's, Q, QF, ()>
where
    QF: Fetch<'w, State = Q::State>,
{
    fn len(&self) -> usize {
        self.query_state
            .matched_archetype_ids
            .iter()
            .map(|id| self.archetypes[*id].len())
            .sum()
    }
}

struct QueryIterationCursor<'w, 's, Q: WorldQuery, QF: Fetch<'w, State = Q::State>, F: WorldQuery> {
    table_id_iter: std::slice::Iter<'s, TableId>,
    archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
    fetch: QF,
    filter: QueryFetch<'w, F>,
    current_len: usize,
    current_index: usize,
    phantom: PhantomData<&'w Q>,
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> Clone for QueryIterationCursor<'w, 's, Q, QF, F>
where
    QF: Fetch<'w, State = Q::State> + Clone,
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

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> QueryIterationCursor<'w, 's, Q, QF, F>
where
    QF: Fetch<'w, State = Q::State>,
{
    const IS_DENSE: bool = QF::IS_DENSE && <QueryFetch<'static, F>>::IS_DENSE;

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
        let fetch = QF::init(
            world,
            &query_state.fetch_state,
            last_change_tick,
            change_tick,
        );
        let filter = QueryFetch::<F>::init(
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
    unsafe fn peek_last(&mut self) -> Option<QF::Item> {
        if self.current_index > 0 {
            if Self::IS_DENSE {
                Some(self.fetch.table_fetch(self.current_index - 1))
            } else {
                Some(self.fetch.archetype_fetch(self.current_index - 1))
            }
        } else {
            None
        }
    }

    // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
    // QueryIterationCursor, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
    #[inline(always)]
    unsafe fn next(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s QueryState<Q, F>,
    ) -> Option<QF::Item> {
        if Self::IS_DENSE {
            loop {
                if self.current_index == self.current_len {
                    let table_id = self.table_id_iter.next()?;
                    let table = &tables[*table_id];
                    self.fetch.set_table(&query_state.fetch_state, table);
                    self.filter.set_table(&query_state.filter_state, table);
                    self.current_len = table.len();
                    self.current_index = 0;
                    continue;
                }

                if !self.filter.table_filter_fetch(self.current_index) {
                    self.current_index += 1;
                    continue;
                }

                let item = self.fetch.table_fetch(self.current_index);

                self.current_index += 1;
                return Some(item);
            }
        } else {
            loop {
                if self.current_index == self.current_len {
                    let archetype_id = self.archetype_id_iter.next()?;
                    let archetype = &archetypes[*archetype_id];
                    self.fetch
                        .set_archetype(&query_state.fetch_state, archetype, tables);
                    self.filter
                        .set_archetype(&query_state.filter_state, archetype, tables);
                    self.current_len = archetype.len();
                    self.current_index = 0;
                    continue;
                }

                if !self.filter.archetype_filter_fetch(self.current_index) {
                    self.current_index += 1;
                    continue;
                }

                let item = self.fetch.archetype_fetch(self.current_index);
                self.current_index += 1;
                return Some(item);
            }
        }
    }
}
