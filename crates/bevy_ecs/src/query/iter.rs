use crate::{
    archetype::{ArchetypeId, Archetypes},
    query::{Fetch, FilterFetch, QueryState, WorldQuery},
    storage::{TableId, Tables},
    world::World,
};
use std::mem::MaybeUninit;

/// An [`Iterator`] over query results of a [`Query`](crate::system::Query).
///
/// This struct is created by the [`Query::iter`](crate::system::Query::iter) and
/// [`Query::iter_mut`](crate::system::Query::iter_mut) methods.
pub struct QueryIter<'w, 's, Q: WorldQuery, F: WorldQuery>
where
    F::Fetch: FilterFetch,
{
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    world: &'w World,
    cursor: QueryIterationCursor<'s, Q, F>,
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> QueryIter<'w, 's, Q, F>
where
    F::Fetch: FilterFetch,
{
    pub(crate) unsafe fn new(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        QueryIter {
            world,
            query_state,
            tables: &world.storages().tables,
            archetypes: &world.archetypes,
            cursor: QueryIterationCursor::init(world, query_state, last_change_tick, change_tick),
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery> Iterator for QueryIter<'w, 's, Q, F>
where
    F::Fetch: FilterFetch,
{
    type Item = <Q::Fetch as Fetch<'w>>::Item;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.cursor
                .next(&self.tables, &self.archetypes, &self.query_state)
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

pub struct QueryPermutationIter<'w, 's, Q: WorldQuery, F: WorldQuery, const K: usize>
where
    F::Fetch: FilterFetch,
{
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    query_state: &'s QueryState<Q, F>,
    world: &'w World,
    cursors: [QueryIterationCursor<'s, Q, F>; K],
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery, const K: usize> QueryPermutationIter<'w, 's, Q, F, K>
where
    F::Fetch: FilterFetch,
{
    pub(crate) unsafe fn new(
        world: &'w World,
        query_state: &'s QueryState<Q, F>,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Self {
        // Initialize array with cursors.
        // There is no FromIterator on arrays, so instead initialize it manually with MaybeUninit

        // MaybeUninit::uninit_array is unstable
        let mut cursors: [MaybeUninit<QueryIterationCursor<'s, Q, F>>; K] =
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

        // MaybeUninit::array_assume_init is unstable
        let cursors: [QueryIterationCursor<'s, Q, F>; K] =
            (&cursors as *const _ as *const [QueryIterationCursor<'s, Q, F>; K]).read();

        QueryPermutationIter {
            world,
            query_state,
            tables: &world.storages().tables,
            archetypes: &world.archetypes,
            cursors,
        }
    }
}

impl<'w, 's, Q: WorldQuery, F: WorldQuery, const K: usize> Iterator
    for QueryPermutationIter<'w, 's, Q, F, K>
where
    F::Fetch: FilterFetch,
    Q::Fetch: Clone,
    F::Fetch: Clone,
{
    type Item = [<Q::Fetch as Fetch<'w>>::Item; K];

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            // first, iterate from last to first until next item is found
            'outer: for i in (0..K).rev() {
                match self.cursors[i].next(&self.tables, &self.archetypes, &self.query_state) {
                    Some(_) => {
                        // walk forward up to last element, propagating cursor state forward
                        for j in (i + 1)..K {
                            self.cursors[j] = self.cursors[j - 1].clone();
                            match self.cursors[j].next(
                                &self.tables,
                                &self.archetypes,
                                &self.query_state,
                            ) {
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

            // MaybeUninit::uninit_array is unstable
            let mut values: [MaybeUninit<<Q::Fetch as Fetch<'w>>::Item>; K] =
                MaybeUninit::uninit().assume_init();

            for (value, cursor) in values.iter_mut().zip(&mut self.cursors) {
                value.as_mut_ptr().write(cursor.peek_last().unwrap());
            }

            // MaybeUninit::array_assume_init is unstable
            let values: [<Q::Fetch as Fetch<'w>>::Item; K] =
                (&values as *const _ as *const [<Q::Fetch as Fetch<'w>>::Item; K]).read();

            Some(values)
        }
    }

    // NOTE: For unfiltered Queries this should actually return a exact size hint,
    // to fulfil the ExactSizeIterator invariant, but this isn't practical without specialization.
    // For more information see Issue #1686.
    fn size_hint(&self) -> (usize, Option<usize>) {
        let max_size: usize = self
            .query_state
            .matched_archetypes
            .ones()
            .map(|index| self.world.archetypes[ArchetypeId::new(index)].len())
            .sum();

        // n! / k!(n-k)! = (n*n-1*...*n-k+1) / k!
        let k_factorial: usize = (1..=K).product();
        let max_permutations =
            (0..K).fold(1, |n, i| n * (max_size.saturating_sub(i))) / k_factorial;

        (0, Some(max_permutations))
    }
}

// NOTE: We can cheaply implement this for unfiltered Queries because we have:
// (1) pre-computed archetype matches
// (2) each archetype pre-computes length
// (3) there are no per-entity filters
// TODO: add an ArchetypeOnlyFilter that enables us to implement this for filters like With<T>
impl<'w, 's, Q: WorldQuery> ExactSizeIterator for QueryIter<'w, 's, Q, ()> {
    fn len(&self) -> usize {
        self.query_state
            .matched_archetypes
            .ones()
            .map(|index| self.world.archetypes[ArchetypeId::new(index)].len())
            .sum()
    }
}

struct QueryIterationCursor<'s, Q: WorldQuery, F: WorldQuery> {
    table_id_iter: std::slice::Iter<'s, TableId>,
    archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
    fetch: Q::Fetch,
    filter: F::Fetch,
    current_len: usize,
    current_index: usize,
    is_dense: bool,
}

impl<'s, Q: WorldQuery, F: WorldQuery> Clone for QueryIterationCursor<'s, Q, F>
where
    Q::Fetch: Clone,
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
            is_dense: self.is_dense,
        }
    }
}

impl<'s, Q: WorldQuery, F: WorldQuery> QueryIterationCursor<'s, Q, F>
where
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
        let fetch = <Q::Fetch as Fetch>::init(
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
            is_dense: fetch.is_dense() && filter.is_dense(),
            fetch,
            filter,
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            current_len: 0,
            current_index: 0,
        }
    }

    /// retreive last returned item again
    #[inline]
    unsafe fn peek_last<'w>(&mut self) -> Option<<Q::Fetch as Fetch<'w>>::Item> {
        if self.current_index > 0 {
            Some(self.fetch.table_fetch(self.current_index - 1))
        } else {
            None
        }
    }

    #[inline]
    unsafe fn next<'w>(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s QueryState<Q, F>,
    ) -> Option<<Q::Fetch as Fetch<'w>>::Item> {
        if self.is_dense {
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
