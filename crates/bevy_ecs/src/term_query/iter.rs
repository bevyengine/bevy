use std::slice;

use crate::{
    archetype::{ArchetypeEntity, ArchetypeId, Archetypes},
    component::Tick,
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{TableId, TableRow, Tables},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{FetchBuffer, FetchedTerm, QueryTerm, QueryTermGroup, Term, TermQueryState, TermState};

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
    filtered: bool,
}

impl<'w, 's> TermQueryCursor<'w, 's> {
    #[inline]
    unsafe fn new<Q: QueryTermGroup>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q>,
    ) -> Self {
        let term_state = query_state.init_term_state(world);
        Self {
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            raw_fetches: FetchBuffer::new(term_state.len()),
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
                if !self.filtered
                    || query_state.filter_fetch(&self.term_state, entity, row, last_run, this_run)
                {
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
                if !self.filtered
                    || query_state.filter_fetch(&self.term_state, entity, row, last_run, this_run)
                {
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
    world: UnsafeWorldCell<'w>,
    last_run: Tick,
    this_run: Tick,
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
            world,
            last_run,
            this_run,
            cursor: TermQueryCursor::new(world, query_state),
        }
    }
}

/// A collection of [`FetchedTerm`] returned from a call to `iter_raw`
pub struct FetchedTerms<'w, 's> {
    world: UnsafeWorldCell<'w>,
    last_run: Tick,
    this_run: Tick,
    terms: &'s Vec<Term>,
    fetches: Vec<FetchedTerm<'w>>,
}

impl<'w, 's> FetchedTerms<'w, 's> {
    /// Returns a reference to the terms used in this fetch
    pub fn terms(&self) -> &'s Vec<Term> {
        self.terms
    }

    /// Casts the term at `index` to type `Q`
    ///
    /// # Safety
    /// - caller must ensure that the term at `index` can be interpreted as type `Q`
    pub unsafe fn cast<Q: QueryTerm>(&self, index: usize) -> Q::Item<'w> {
        Q::from_fetch(
            self.world,
            self.last_run,
            self.this_run,
            &self.fetches[index],
        )
    }

    /// Casts terms starting at `index` to type `Q`
    ///
    /// # Safety
    /// - caller must ensure that the terms starting at `index` can be interpreted as type `Q`
    pub unsafe fn cast_many<Q: QueryTermGroup>(&self, index: usize) -> Q::Item<'w> {
        Q::from_fetches(
            self.world,
            self.last_run,
            self.this_run,
            &mut self.fetches[index..].iter(),
        )
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
                .map(|fetches| FetchedTerms {
                    world: self.world,
                    last_run: self.last_run,
                    this_run: self.this_run,
                    terms: &self.query_state.terms,
                    fetches: fetches.to_vec(),
                })
        }
    }
}

/// An [`Iterator`] over query results of a [`TermQuery`](crate::system::TermQuery).
///
/// This struct is created by the [`TermQuery::iter`](crate::system::TermQuery::iter) and
/// [`TermQuery::iter_mut`](crate::system::TermQuery::iter_mut) methods.
pub struct TermQueryIter<'w, 's, Q: QueryTermGroup> {
    query_state: &'s TermQueryState<Q>,
    world: UnsafeWorldCell<'w>,
    last_run: Tick,
    this_run: Tick,
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
            world,
            last_run,
            this_run,
            cursor: TermQueryCursor::new(world, query_state),
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
            self.cursor
                .next(
                    &self.world.storages().tables,
                    self.world.archetypes(),
                    self.query_state,
                    self.last_run,
                    self.this_run,
                )
                .map(|fetches| {
                    Q::from_fetches(
                        self.world,
                        self.last_run,
                        self.this_run,
                        &mut fetches.iter(),
                    )
                })
        }
    }
}
