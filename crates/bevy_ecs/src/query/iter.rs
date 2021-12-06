use crate::{
    archetype::{ArchetypeId, Archetypes},
    query::{Fetch, FilterFetch, QueryState, ReadOnlyFetch, WorldQuery},
    storage::{TableId, Tables},
    world::World,
};
use std::{marker::PhantomData, mem::MaybeUninit};

/// An [`Iterator`] over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::iter`](crate::system::Query::iter) and
/// [`Query::iter_mut`](crate::system::Query::iter_mut) methods.
pub struct QueryIter<'w, 's, Q: WorldQuery, QF: Fetch<'w, 's, State = Q::State>, F: WorldQuery>
where
    F::Fetch: FilterFetch,
{
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    world: &'w World,
    table_id_iter: std::slice::Iter<'s, TableId>,
    archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
    fetch: QF,
    filter: F::Fetch,
    current_len: usize,
    current_index: usize,
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> QueryIter<'w, 's, Q, QF, F>
where
    F::Fetch: FilterFetch,
    QF: Fetch<'w, 's, State = Q::State>,
{
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `query_state.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsound.
    pub(crate) unsafe fn new(
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
        let filter = <F::Fetch as Fetch>::init(
            world,
            &query_state.filter_state,
            last_change_tick,
            change_tick,
        );

        QueryIter {
            world,
            query_state,
            tables: &world.storages().tables,
            archetypes: &world.archetypes,
            fetch,
            filter,
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            current_len: 0,
            current_index: 0,
        }
    }
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> Iterator for QueryIter<'w, 's, Q, QF, F>
where
    F::Fetch: FilterFetch,
    QF: Fetch<'w, 's, State = Q::State>,
{
    type Item = QF::Item;

    // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
    // QueryIter, QueryIterationCursor, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
    // We can't currently reuse QueryIterationCursor in QueryIter for performance reasons. See #1763 for context.
    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if QF::IS_DENSE && F::Fetch::IS_DENSE {
                loop {
                    if self.current_index == self.current_len {
                        let table_id = self.table_id_iter.next()?;
                        let table = &self.tables[*table_id];
                        self.fetch.set_table(&self.query_state.fetch_state, table);
                        self.filter.set_table(&self.query_state.filter_state, table);
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
                        let archetype = &self.archetypes[*archetype_id];
                        self.fetch.set_archetype(
                            &self.query_state.fetch_state,
                            archetype,
                            self.tables,
                        );
                        self.filter.set_archetype(
                            &self.query_state.filter_state,
                            archetype,
                            self.tables,
                        );
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

    // NOTE: For unfiltered Queries this should actually return a exact size hint,
    // to fulfil the ExactSizeIterator invariant, but this isn't practical without specialization.
    // For more information see Issue #1686.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let max_size = self
            .query_state
            .matched_archetypes
            .ones()
            .map(|index| self.world.archetypes[ArchetypeId::new(index)].len())
            .sum();

        (0, Some(max_size))
    }
}

pub struct QueryCombinationIter<'w, 's, Q: WorldQuery, QF, F: WorldQuery, const K: usize>
where
    QF: Fetch<'w, 's, State = Q::State>,
    F::Fetch: FilterFetch,
{
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    world: &'w World,
    cursors: [QueryIterationCursor<'w, 's, Q, QF, F>; K],
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery, const K: usize>
    QueryCombinationIter<'w, 's, Q, QF, F, K>
where
    QF: Fetch<'w, 's, State = Q::State>,
    F::Fetch: FilterFetch,
{
    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `query_state.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsound.
    pub(crate) unsafe fn new(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        // Initialize array with cursors.
        // There is no FromIterator on arrays, so instead initialize it manually with MaybeUninit

        // TODO: use MaybeUninit::uninit_array if it stabilizes
        let mut cursors: [MaybeUninit<QueryIterationCursor<'w, 's, Q, QF, F>>; K] =
            MaybeUninit::uninit().assume_init();
        for (i, cursor) in cursors.iter_mut().enumerate() {
            match i {
                0 => cursor.as_mut_ptr().write(QueryIterationCursor::init(
                    world,
                    query_state,
                    last_change_tick,
                    change_tick,
                )),
                _ => cursor.as_mut_ptr().write(QueryIterationCursor::init_empty(
                    world,
                    query_state,
                    last_change_tick,
                    change_tick,
                )),
            }
        }

        // TODO: use MaybeUninit::array_assume_init if it stabilizes
        let cursors: [QueryIterationCursor<'w, 's, Q, QF, F>; K] =
            (&cursors as *const _ as *const [QueryIterationCursor<'w, 's, Q, QF, F>; K]).read();

        QueryCombinationIter {
            world,
            query_state,
            tables: &world.storages().tables,
            archetypes: &world.archetypes,
            cursors,
        }
    }

    /// Safety:
    /// The lifetime here is not restrictive enough for Fetch with &mut access,
    /// as calling `fetch_next_aliased_unchecked` multiple times can produce multiple
    /// references to the same component, leading to unique reference aliasing.
    ///.
    /// It is always safe for shared access.
    unsafe fn fetch_next_aliased_unchecked(&mut self) -> Option<[QF::Item; K]>
    where
        QF: Clone,
        F::Fetch: Clone,
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

        // TODO: use MaybeUninit::uninit_array if it stabilizes
        let mut values: [MaybeUninit<QF::Item>; K] = MaybeUninit::uninit().assume_init();

        for (value, cursor) in values.iter_mut().zip(&mut self.cursors) {
            value.as_mut_ptr().write(cursor.peek_last().unwrap());
        }

        // TODO: use MaybeUninit::array_assume_init if it stabilizes
        let values: [QF::Item; K] = (&values as *const _ as *const [QF::Item; K]).read();

        Some(values)
    }

    /// Get next combination of queried components
    #[inline]
    pub fn fetch_next(&mut self) -> Option<[QF::Item; K]>
    where
        QF: Clone,
        F::Fetch: Clone,
    {
        // safety: we are limiting the returned reference to self,
        // making sure this method cannot be called multiple times without getting rid
        // of any previously returned unique references first, thus preventing aliasing.
        unsafe { self.fetch_next_aliased_unchecked() }
    }
}

// Iterator type is intentionally implemented only for read-only access.
// Doing so for mutable references would be unsound, because  calling `next`
// multiple times would allow multiple owned references to the same data to exist.
impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery, const K: usize> Iterator
    for QueryCombinationIter<'w, 's, Q, QF, F, K>
where
    QF: Fetch<'w, 's, State = Q::State> + Clone + ReadOnlyFetch,
    F::Fetch: Clone + FilterFetch + ReadOnlyFetch,
{
    type Item = [QF::Item; K];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // Safety: it is safe to alias for ReadOnlyFetch
        unsafe { QueryCombinationIter::fetch_next_aliased_unchecked(self) }
    }

    // NOTE: For unfiltered Queries this should actually return a exact size hint,
    // to fulfil the ExactSizeIterator invariant, but this isn't practical without specialization.
    // For more information see Issue #1686.
    fn size_hint(&self) -> (usize, Option<usize>) {
        if K == 0 {
            return (0, Some(0));
        }

        let max_size: usize = self
            .query_state
            .matched_archetypes
            .ones()
            .map(|index| self.world.archetypes[ArchetypeId::new(index)].len())
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

        (0, max_combinations)
    }
}

// NOTE: We can cheaply implement this for unfiltered Queries because we have:
// (1) pre-computed archetype matches
// (2) each archetype pre-computes length
// (3) there are no per-entity filters
// TODO: add an ArchetypeOnlyFilter that enables us to implement this for filters like With<T>
impl<'w, 's, Q: WorldQuery, QF> ExactSizeIterator for QueryIter<'w, 's, Q, QF, ()>
where
    QF: Fetch<'w, 's, State = Q::State>,
{
    fn len(&self) -> usize {
        self.query_state
            .matched_archetypes
            .ones()
            .map(|index| self.world.archetypes[ArchetypeId::new(index)].len())
            .sum()
    }
}

struct QueryIterationCursor<
    'w,
    's,
    Q: WorldQuery,
    QF: Fetch<'w, 's, State = Q::State>,
    F: WorldQuery,
> {
    table_id_iter: std::slice::Iter<'s, TableId>,
    archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
    fetch: QF,
    filter: F::Fetch,
    current_len: usize,
    current_index: usize,
    phantom: PhantomData<&'w Q>,
}

impl<'w, 's, Q: WorldQuery, QF, F: WorldQuery> Clone for QueryIterationCursor<'w, 's, Q, QF, F>
where
    QF: Fetch<'w, 's, State = Q::State> + Clone,
    F::Fetch: Clone,
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
    QF: Fetch<'w, 's, State = Q::State>,
    F::Fetch: FilterFetch,
{
    unsafe fn init_empty(
        world: &World,
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
        world: &World,
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
        let filter = <F::Fetch as Fetch>::init(
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
            if QF::IS_DENSE && F::Fetch::IS_DENSE {
                Some(self.fetch.table_fetch(self.current_index - 1))
            } else {
                Some(self.fetch.archetype_fetch(self.current_index - 1))
            }
        } else {
            None
        }
    }

    // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
    // QueryIter, QueryIterationCursor, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
    // We can't currently reuse QueryIterationCursor in QueryIter for performance reasons. See #1763 for context.
    #[inline(always)]
    unsafe fn next(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s QueryState<Q, F>,
    ) -> Option<QF::Item> {
        if QF::IS_DENSE && F::Fetch::IS_DENSE {
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
