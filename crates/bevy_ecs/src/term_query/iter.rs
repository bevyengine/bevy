use std::{marker::PhantomData, slice};

use crate::{
    archetype::{ArchetypeEntity, ArchetypeId, Archetypes},
    component::Tick,
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{TableId, TableRow, Tables},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{FetchedTerm, QueryTermGroup, RawFetches, TermQueryState, TermState, TermVec};

pub struct TermQueryCursor<'w, 's> {
    table_id_iter: slice::Iter<'s, TableId>,
    archetype_id_iter: slice::Iter<'s, ArchetypeId>,
    raw_fetches: RawFetches,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    term_state: TermVec<TermState<'w>>,
    current_len: usize,
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
            raw_fetches: RawFetches::new(term_state.len()),
            table_entities: &[],
            archetype_entities: &[],
            dense: term_state.iter().all(|t| t.dense()),
            term_state,
            current_len: 0,
            current_row: 0,
        }
    }

    #[inline(always)]
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

                // SAFETY:
                // - set_table was called prior.
                // - `current_row` must be a table row in range of the current table,
                //   because if it was not, then the if above would have been executed.
                // - fetch is only called once for each `entity`.
                self.current_row += 1;

                if query_state.filter_fetch(&self.term_state, entity, row) {
                    query_state.fetch(&self.term_state, entity, row, self.raw_fetches.as_uninit());
                    return Some(self.raw_fetches.as_slice());
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
                    query_state.set_table(&mut self.term_state, table);
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
                // Apply filters
                if query_state.filter_fetch(&mut self.term_state, entity, row) {
                    query_state.fetch(&self.term_state, entity, row, self.raw_fetches.as_uninit());
                    return Some(self.raw_fetches.as_slice());
                }
            }
        }
    }
}

pub struct TermQueryIterUntyped<'w, 's> {
    query_state: &'s TermQueryState<()>,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    cursor: TermQueryCursor<'w, 's>,
}

impl<'w, 's> TermQueryIterUntyped<'w, 's> {
    #[inline]
    pub unsafe fn new<Q: QueryTermGroup>(
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

// This would be a streaming iterator if rust supported those
impl<'w, 's> TermQueryIterUntyped<'w, 's> {
    #[inline(always)]
    fn next_fetch<'f>(&'f mut self) -> Option<&'f [FetchedTerm<'w>]> {
        unsafe {
            self.cursor
                .next(self.tables, self.archetypes, self.query_state)
        }
    }
}

// Slower iterator API that clones fetches
impl<'w, 's> Iterator for TermQueryIterUntyped<'w, 's> {
    type Item = Vec<FetchedTerm<'w>>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        self.next_fetch().map(|fetches| fetches.to_vec())
    }
}

pub struct TermQueryIter<'w, 's, Q: QueryTermGroup> {
    inner: TermQueryIterUntyped<'w, 's>,
    _marker: PhantomData<Q>,
}

impl<'w, 's, Q: QueryTermGroup> TermQueryIter<'w, 's, Q> {
    #[inline(always)]
    pub unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        Self {
            inner: TermQueryIterUntyped::new(world, query_state, last_run, this_run),
            _marker: PhantomData::default(),
        }
    }
}

impl<'w, 's, Q: QueryTermGroup> Iterator for TermQueryIter<'w, 's, Q> {
    type Item = Q::Item<'w>;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if let Some(fetches) = &mut self.inner.next_fetch() {
                Some(Q::from_fetches(&mut fetches.iter()))
            } else {
                None
            }
        }
    }
}
