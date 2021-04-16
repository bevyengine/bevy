use crate::{
    archetype::{ArchetypeId, Archetypes},
    query::{Fetch, FilterFetch, QueryState, WorldQuery},
    storage::{TableId, Tables},
    world::World,
};

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
    table_id_iter: std::slice::Iter<'s, TableId>,
    archetype_id_iter: std::slice::Iter<'s, ArchetypeId>,
    fetch: Q::Fetch,
    filter: F::Fetch,
    is_dense: bool,
    current_len: usize,
    current_index: usize,
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
        QueryIter {
            is_dense: fetch.is_dense() && filter.is_dense(),
            world,
            query_state,
            fetch,
            filter,
            tables: &world.storages().tables,
            archetypes: &world.archetypes,
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            current_len: 0,
            current_index: 0,
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
            if self.is_dense {
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
