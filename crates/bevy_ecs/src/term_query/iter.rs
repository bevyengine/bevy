use std::{ops::Range, slice};

use crate::{
    archetype::{ArchetypeEntity, ArchetypeId, Archetypes},
    component::Tick,
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{TableId, TableRow, Tables},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{QueryTermGroup, Term, TermQueryState, TermState};

struct TermQueryCursorUntyped<'w, 's> {
    table_id_iter: slice::Iter<'s, TableId>,
    archetype_id_iter: slice::Iter<'s, ArchetypeId>,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    term_state: Vec<TermState<'w>>,
    // length of the table table or length of the archetype, depending on whether all terms are dense
    current_len: usize,
    // either table row or archetype index, depending on whether all terms are dense
    current_row: usize,
    dense: bool,
    filtered: bool,
}

impl<'w, 's> TermQueryCursorUntyped<'w, 's> {
    #[inline]
    unsafe fn new<Q: QueryTermGroup>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
    ) -> Self {
        let term_state = query_state.init_term_state(world);
        Self {
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            table_entities: &[],
            archetype_entities: &[],
            term_state,
            current_len: 0,
            current_row: 0,
            dense: query_state.dense,
            filtered: query_state.filtered,
        }
    }

    #[inline]
    unsafe fn next<Q: QueryTermGroup>(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s TermQueryState<Q>,
        last_run: Tick,
        this_run: Tick,
    ) -> Option<(Entity, TableRow, Vec<TermState<'w>>)> {
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
                if !self.filtered
                    || query_state.filter_fetch(&self.term_state, entity, row, last_run, this_run)
                {
                    return Some((entity, row, self.term_state.clone()));
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
                if !self.filtered
                    || query_state.filter_fetch(&self.term_state, entity, row, last_run, this_run)
                {
                    return Some((entity, row, self.term_state.clone()));
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
    world: UnsafeWorldCell<'w>,
    last_run: Tick,
    this_run: Tick,
    cursor: TermQueryCursorUntyped<'w, 's>,
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
            world,
            last_run,
            this_run,
            cursor: TermQueryCursorUntyped::new(world, query_state),
        }
    }
}

/// A collection of [`FetchedTerm`] returned from a call to `iter_raw`
pub struct FetchedTerms<'w, 's> {
    world: UnsafeWorldCell<'w>,
    last_run: Tick,
    this_run: Tick,
    entity: Entity,
    table_row: TableRow,
    terms: &'s Vec<Term>,
    state: Vec<TermState<'w>>,
}

impl<'w, 's> FetchedTerms<'w, 's> {
    /// Returns a reference to the terms used in this fetch
    pub fn terms(&self) -> &'s Vec<Term> {
        self.terms
    }

    /// Returns an iterator across tuples of [`Term`] and [`FetchedTerm`]
    pub fn iter_terms(&self) -> impl Iterator<Item = (&Term, &TermState<'w>)> {
        self.terms.iter().zip(self.state.iter())
    }

    pub unsafe fn fetch<Q: QueryTermGroup>(&self, index: usize) -> Q::Item<'w> {
        self.fetch_range::<Q>(index..index + 1)
    }

    pub unsafe fn fetch_state<Q: QueryTermGroup>(&self, state: &TermState<'w>) -> Q::Item<'w> {
        self.fetch_slice::<Q>(slice::from_ref(state))
    }

    pub unsafe fn fetch_slice<Q: QueryTermGroup>(&self, state: &[TermState<'w>]) -> Q::Item<'w> {
        Q::fetch_terms(
            self.world,
            self.last_run,
            self.this_run,
            &mut state.iter(),
            self.entity,
            self.table_row,
        )
    }

    pub unsafe fn fetch_range<Q: QueryTermGroup>(&self, range: Range<usize>) -> Q::Item<'w> {
        self.fetch_slice::<Q>(&self.state[range])
    }
}

impl<'w, 's> Iterator for TermQueryIterUntyped<'w, 's> {
    type Item = FetchedTerms<'w, 's>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY:
        // `tables` and `archetypes` belong to the same world that the cursor was initialized for.
        // `query_state` is the state that was passed to `TermQueryCursor::new`.
        unsafe {
            self.cursor
                .next(
                    &self.world.storages().tables,
                    self.world.archetypes(),
                    self.query_state,
                    self.last_run,
                    self.this_run,
                )
                .map(|(entity, table_row, state)| FetchedTerms {
                    entity,
                    table_row,
                    world: self.world,
                    last_run: self.last_run,
                    this_run: self.this_run,
                    terms: &self.query_state.terms,
                    state,
                })
        }
    }
}

pub struct TermQueryIter<'w, 's, Q: QueryTermGroup> {
    world: UnsafeWorldCell<'w>,
    table_id_iter: slice::Iter<'s, TableId>,
    archetype_id_iter: slice::Iter<'s, ArchetypeId>,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    query_state: &'s TermQueryState<Q>,
    term_state: Vec<TermState<'w>>,
    // length of the table table or length of the archetype, depending on whether all terms are dense
    current_len: usize,
    // either table row or archetype index, depending on whether all terms are dense
    current_row: usize,
    last_run: Tick,
    this_run: Tick,
}

impl<'w, 's, Q: QueryTermGroup> TermQueryIter<'w, 's, Q> {
    #[inline]
    pub unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        let term_state = query_state.init_term_state(world);
        Self {
            world,
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            table_entities: &[],
            archetype_entities: &[],
            query_state,
            term_state,
            current_len: 0,
            current_row: 0,
            last_run,
            this_run,
        }
    }
}

impl<'w, 's, Q: QueryTermGroup> Iterator for TermQueryIter<'w, 's, Q> {
    type Item = Q::Item<'w>;

    #[inline(always)]
    fn next(&mut self) -> Option<Q::Item<'w>> {
        unsafe {
            let tables = &self.world.storages().tables;
            let archetypes = self.world.archetypes();
            if Q::DENSE || self.query_state.dense {
                loop {
                    // we are on the beginning of the query, or finished processing a table, so skip to the next
                    if self.current_row == self.current_len {
                        let table_id = self.table_id_iter.next()?;
                        let table = tables.get(*table_id).debug_checked_unwrap();
                        // SAFETY: `table` is from the world that `fetch/filter` were created for,
                        // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                        if self.query_state.filtered {
                            self.query_state.set_table(&mut self.term_state, table)
                        } else {
                            Q::set_tables(&mut self.term_state.iter_mut(), table);
                        }
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
                    if !self.query_state.filtered
                        || self.query_state.filter_fetch(
                            &self.term_state,
                            entity,
                            row,
                            self.last_run,
                            self.this_run,
                        )
                    {
                        return Some(Q::fetch_terms(
                            self.world,
                            self.last_run,
                            self.this_run,
                            &mut self.term_state.iter(),
                            entity,
                            row,
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
                        if self.query_state.filtered {
                            self.query_state
                                .set_archetype(&mut self.term_state, archetype, table)
                        } else {
                            Q::set_archetypes(&mut self.term_state.iter_mut(), archetype, table);
                        }
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
                    if !self.query_state.filtered
                        || self.query_state.filter_fetch(
                            &self.term_state,
                            entity,
                            row,
                            self.last_run,
                            self.this_run,
                        )
                    {
                        return Some(Q::fetch_terms(
                            self.world,
                            self.last_run,
                            self.this_run,
                            &mut self.term_state.iter(),
                            entity,
                            row,
                        ));
                    }
                }
            }
        }
    }
}
