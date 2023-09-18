use std::slice;

use crate::{
    archetype::{ArchetypeEntity, ArchetypeId, Archetypes},
    component::Tick,
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{TableId, TableRow, Tables},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{FetchBuffer, FetchedTerm, QueryTermGroup, Term, TermQueryState, TermState};

struct TermQueryCursor<'w, 's> {
    table_id_iter: slice::Iter<'s, TableId>,
    archetype_id_iter: slice::Iter<'s, ArchetypeId>,
    raw_fetches: FetchBuffer,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    term_state: Vec<TermState<'w>>,
    // length of the table table or length of the archetype, depending on whether all terms are dense
    current_len: usize,
    // either table row or archetype index, depending on whether all terms are dense
    current_row: usize,
    dense: bool,
}

impl<'w, 's> TermQueryCursor<'w, 's> {
    #[inline]
    unsafe fn new<Q: QueryTermGroup>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        let term_state = query_state.init_term_state(world, last_run, this_run);
        Self {
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            raw_fetches: FetchBuffer::new(term_state.len()),
            table_entities: &[],
            archetype_entities: &[],
            dense: term_state.iter().all(|t| t.dense()),
            term_state,
            current_len: 0,
            current_row: 0,
        }
    }

    #[inline]
    unsafe fn next<Q: QueryTermGroup>(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s TermQueryState<Q>,
    ) -> Option<&[FetchedTerm<'w>]> {
        if self.dense {
            loop {
                // we are on the beginning of the query, or finished processing a table, so skip to the next
                if self.current_row == self.current_len {
                    let table_id = self.table_id_iter.next()?;
                    let table = tables.get(*table_id).debug_checked_unwrap();
                    // SAFETY: `table` is from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    query_state.set_table(&mut self.term_state, table);
                    self.table_entities = table.entities();
                    self.current_len = table.entity_count();
                    self.current_row = 0;
                    continue;
                }

                // SAFETY: set_table was called prior.
                // `current_row` is a table row in range of the current table, because if it was not, then the if above would have been executed.
                let entity = *self.table_entities.get_unchecked(self.current_row);
                let row = TableRow::new(self.current_row);
                self.current_row += 1;

                // SAFETY:
                // - set_table was called prior.
                // - `current_row` must be a table row in range of the current table,
                //   because if it was not, then the if above would have been executed.
                // - fetch is only called once for each `entity`.
                if query_state.filter_fetch(&self.term_state, entity, row) {
                    return Some(query_state.fetch(
                        &self.term_state,
                        entity,
                        row,
                        self.raw_fetches.as_uninit(),
                    ));
                }
            }
        } else {
            loop {
                if self.current_row == self.current_len {
                    let archetype_id = self.archetype_id_iter.next()?;
                    let archetype = archetypes.get(*archetype_id).debug_checked_unwrap();
                    // SAFETY: `archetype` and `tables` are from the world that `fetch/filter` were created for,
                    // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                    let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                    query_state.set_archetype(&mut self.term_state, archetype, table);
                    self.archetype_entities = archetype.entities();
                    self.current_len = archetype.len();
                    self.current_row = 0;
                    continue;
                }

                // SAFETY:
                // - set_archetype was called prior.
                // - `current_row` must be an archetype index row in range of the current archetype,
                //   because if it was not, then the if above would have been executed.
                // - fetch is only called once for each `archetype_entity`.
                let archetype_entity = self.archetype_entities.get_unchecked(self.current_row);
                self.current_row += 1;

                let entity = archetype_entity.entity();
                let row = archetype_entity.table_row();
                if query_state.filter_fetch(&mut self.term_state, entity, row) {
                    return Some(query_state.fetch(
                        &self.term_state,
                        entity,
                        row,
                        self.raw_fetches.as_uninit(),
                    ));
                }
            }
        }
    }
}

/// An untyped [`Iterator`] over query results of a [`TermQuery`](crate::system::TermQuery).
///
/// This struct is created by the [`TermQuery::iter_raw`](crate::system::TermQuery::iter_raw) method.
pub struct TermQueryIterUntyped<'w, 's> {
    query_state: &'s TermQueryState<()>,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    cursor: TermQueryCursor<'w, 's>,
}

impl<'w, 's> TermQueryIterUntyped<'w, 's> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    #[inline]
    pub(crate) unsafe fn new<Q: QueryTermGroup>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            query_state: query_state.transmute_ref(),
            tables: &world.storages().tables,
            archetypes: world.archetypes(),
            cursor: TermQueryCursor::new(world, query_state, last_run, this_run),
        }
    }
}

impl<'w, 's> Iterator for TermQueryIterUntyped<'w, 's> {
    type Item = (&'s Vec<Term>, Vec<FetchedTerm<'w>>);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.cursor
                .next(self.tables, self.archetypes, self.query_state)
                .map(|fetches| (&self.query_state.terms, fetches.to_vec()))
        }
    }
}

/// An [`Iterator`] over query results of a [`TermQuery`](crate::system::TermQuery).
///
/// This struct is created by the [`TermQuery::iter`](crate::system::TermQuery::iter) and
/// [`TermQuery::iter_mut`](crate::system::TermQuery::iter_mut) methods.
pub struct TermQueryIter<'w, 's, Q: QueryTermGroup> {
    query_state: &'s TermQueryState<Q>,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    cursor: TermQueryCursor<'w, 's>,
}

impl<'w, 's, Q: QueryTermGroup> TermQueryIter<'w, 's, Q> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    #[inline]
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            query_state,
            tables: &world.storages().tables,
            archetypes: world.archetypes(),
            cursor: TermQueryCursor::new(world, query_state, last_run, this_run),
        }
    }
}

impl<'w, 's, Q: QueryTermGroup> Iterator for TermQueryIter<'w, 's, Q> {
    type Item = Q::Item<'w>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // `tables` and `archetypes` belong to the same world that the cursor was initialized for.
        // `query_state` is the state that was passed to `QueryIterationCursor::init`.
        unsafe {
            if let Some(fetches) = self
                .cursor
                .next(self.tables, self.archetypes, self.query_state)
            {
                Some(Q::from_fetches(&mut fetches.iter()))
            } else {
                None
            }
        }
    }
}
