use std::{marker::PhantomData, slice};

use crate::{
    archetype::{ArchetypeEntity, ArchetypeId, Archetypes},
    component::Tick,
    query::DebugCheckedUnwrap,
    storage::Tables,
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{Fetchable, FetchedTerm, QueryTermGroup, TermQueryState, TermState};

pub struct TermQueryCursor<'w, 's> {
    archetype_id_iter: slice::Iter<'s, ArchetypeId>,
    archetype_entities: &'w [ArchetypeEntity],
    term_state: Vec<TermState<'w>>,
    current_len: usize,
    current_row: usize,
}

impl<'w, 's> TermQueryCursor<'w, 's> {
    unsafe fn new<Q: QueryTermGroup>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        let term_state = query_state.init_term_state(world, last_run, this_run);
        Self {
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            archetype_entities: &[],
            term_state,
            current_len: 0,
            current_row: 0,
        }
    }

    unsafe fn next<Q: QueryTermGroup>(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s TermQueryState<Q>,
    ) -> Option<Vec<FetchedTerm<'w>>> {
        loop {
            if self.current_row == self.current_len {
                let archetype_id = self.archetype_id_iter.next()?;
                let archetype = archetypes.get(*archetype_id).debug_checked_unwrap();
                // SAFETY: `archetype` and `tables` are from the world that `fetch/filter` were created for,
                // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                query_state
                    .terms
                    .iter()
                    .zip(self.term_state.iter_mut())
                    .for_each(|(term, state)| term.set_table(state, table));
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
            if query_state
                .terms
                .iter()
                .zip(self.term_state.iter_mut())
                .all(|(term, state)| term.filter_fetch(state, entity, row))
            {
                return Some(
                    query_state
                        .terms
                        .iter()
                        .zip(self.term_state.iter_mut())
                        .map(|(term, state)| term.fetch(state, entity, row))
                        .collect(),
                );
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

impl<'w, 's> Iterator for TermQueryIterUntyped<'w, 's> {
    type Item = Vec<FetchedTerm<'w>>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.cursor
                .next(self.tables, self.archetypes, self.query_state)
        }
    }
}

pub struct TermQueryIter<'w, 's, Q: QueryTermGroup> {
    inner: TermQueryIterUntyped<'w, 's>,
    _marker: PhantomData<Q>,
}

impl<'w, 's, Q: QueryTermGroup> TermQueryIter<'w, 's, Q> {
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

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.inner
                .next()
                .map(|fetches| Q::from_fetches(&mut fetches.into_iter()))
        }
    }
}
