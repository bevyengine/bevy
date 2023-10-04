use std::{marker::PhantomData, mem::MaybeUninit, ops::Range, slice};

use bevy_ptr::ThinSlicePtr;

use crate::{
    archetype::{ArchetypeEntity, ArchetypeId, Archetypes},
    component::Tick,
    entity::Entity,
    query::DebugCheckedUnwrap,
    storage::{TableId, TableRow, Tables},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{QueryFetchGroup, Term, TermQueryState, TermState};

struct TermQueryCursorUntyped<'w, 's> {
    table_id_iter: slice::Iter<'s, TableId>,
    archetype_id_iter: slice::Iter<'s, ArchetypeId>,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    fetch_state: Vec<TermState<'w>>,
    filter_state: Vec<TermState<'w>>,
    // length of the table table or length of the archetype, depending on whether all terms are dense
    current_len: usize,
    // either table row or archetype index, depending on whether all terms are dense
    current_row: usize,
    dense: bool,
    filtered: bool,
}

impl<'w, 's> TermQueryCursorUntyped<'w, 's> {
    #[inline]
    unsafe fn new<Q: QueryFetchGroup, F: QueryFetchGroup>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q, F>,
    ) -> Self {
        let fetch_state = query_state.init_fetch_state(world);
        let filter_state = query_state.init_filter_state(world);
        let dense = fetch_state.iter().all(|t| t.dense());
        Self {
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            table_entities: &[],
            archetype_entities: &[],
            fetch_state,
            filter_state,
            current_len: 0,
            current_row: 0,
            dense,
            filtered: query_state.filter_terms.iter().any(|t| t.filtered()),
        }
    }

    #[inline]
    unsafe fn next<Q: QueryFetchGroup, F: QueryFetchGroup>(
        &mut self,
        tables: &'w Tables,
        archetypes: &'w Archetypes,
        query_state: &'s TermQueryState<Q, F>,
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
                    query_state.set_table(&mut self.fetch_state, &mut self.filter_state, table);
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
                    || query_state.filter_fetch(&self.filter_state, entity, row, last_run, this_run)
                {
                    return Some((entity, row, self.fetch_state.clone()));
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
                    query_state.set_archetype(
                        &mut self.fetch_state,
                        &mut self.filter_state,
                        archetype,
                        table,
                    );
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
                    || query_state.filter_fetch(&self.filter_state, entity, row, last_run, this_run)
                {
                    return Some((entity, row, self.fetch_state.clone()));
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
    pub(crate) unsafe fn new<Q: QueryFetchGroup, F: QueryFetchGroup>(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q, F>,
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

/// A collection of [`TermState`] returned from a call to `iter_raw`
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

    /// Returns an iterator across tuples of [`Term`] and [`TermState`]
    pub fn iter(&self) -> impl Iterator<Item = (&Term, &TermState<'w>)> {
        self.terms.iter().zip(self.state.iter())
    }

    /// Returns `Q::Item` constructed from the given [`TermState`]
    ///
    /// # Safety
    /// - [`TermState`] at `index` must be fetcheable as `Q::Item`
    pub unsafe fn fetch<Q: QueryFetchGroup>(&self, index: usize) -> Q::Item<'w> {
        self.fetch_range::<Q>(index..index + 1)
    }

    /// Returns `Q::Item` constructed from the given [`TermState`]
    ///
    /// # Safety
    /// - [`TermState`] must be fetcheable as `Q::Item`
    pub unsafe fn fetch_state<Q: QueryFetchGroup>(&self, state: &TermState<'w>) -> Q::Item<'w> {
        self.fetch_slice::<Q>(slice::from_ref(state))
    }

    /// Returns `Q::Item` constructed from the given slice of [`TermState`]
    ///
    /// # Safety
    /// - [`TermState`] in slice must be fetcheable as `Q::Item`
    pub unsafe fn fetch_slice<Q: QueryFetchGroup>(&self, state: &[TermState<'w>]) -> Q::Item<'w> {
        Q::fetch_terms(
            self.world,
            self.last_run,
            self.this_run,
            ThinSlicePtr::from(state),
            self.entity,
            self.table_row,
        )
    }

    /// Returns `Q::Item` constructed from the given range of indices
    ///
    /// # Safety
    /// - [`TermState`] in given range must be fetcheable as `Q::Item`
    pub unsafe fn fetch_range<Q: QueryFetchGroup>(&self, range: Range<usize>) -> Q::Item<'w> {
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
                    terms: &self.query_state.fetch_terms,
                    state,
                })
        }
    }
}

/// An [`Iterator`] over query results of a [`TermQuery`](crate::system::TermQuery).
///
/// This struct is created by the [`TermQuery::iter`](crate::system::TermQuery::iter) and
/// [`TermQuery::iter_mut`](crate::system::TermQuery::iter_mut) methods.
pub struct TermQueryIter<'w, 's, Q: QueryFetchGroup, F: QueryFetchGroup> {
    world: UnsafeWorldCell<'w>,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    table_id_iter: slice::Iter<'s, TableId>,
    archetype_id_iter: slice::Iter<'s, ArchetypeId>,
    table_entities: &'w [Entity],
    archetype_entities: &'w [ArchetypeEntity],
    // With generic_const_expr this can be [_; Q::SIZE] and eliminate the fallback
    fetch_state: MaybeUninit<[TermState<'w>; 16]>,
    filter_state: MaybeUninit<[TermState<'w>; 16]>,
    fetch_state_vec: Vec<TermState<'w>>,
    filter_state_vec: Vec<TermState<'w>>,
    // length of the table table or length of the archetype, depending on whether all terms are dense
    current_len: usize,
    // either table row or archetype index, depending on whether all terms are dense
    current_row: usize,
    last_run: Tick,
    this_run: Tick,
    _marker: PhantomData<(Q, F)>,
}

impl<'w, 's, Q: QueryFetchGroup, F: QueryFetchGroup> TermQueryIter<'w, 's, Q, F> {
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    #[inline]
    pub unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s TermQueryState<Q, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> Self {
        let mut fetch_state_vec = query_state.init_fetch_state(world);
        let mut filter_state_vec = query_state.init_filter_state(world);
        let mut fetch_state: MaybeUninit<[TermState<'_>; 16]> = MaybeUninit::uninit();
        let mut filter_state: MaybeUninit<[TermState<'_>; 16]> = MaybeUninit::uninit();
        if Q::SIZE <= 16 {
            for (i, term) in fetch_state_vec.into_iter().enumerate() {
                fetch_state.assume_init_mut()[i] = term;
            }
            fetch_state_vec = Vec::new();
        }
        if F::SIZE <= 16 {
            for (i, term) in filter_state_vec.into_iter().enumerate() {
                filter_state.assume_init_mut()[i] = term;
            }
            filter_state_vec = Vec::new();
        }
        let tables = &world.storages().tables;
        let archetypes = world.archetypes();

        Self {
            world,
            tables,
            archetypes,
            table_id_iter: query_state.matched_table_ids.iter(),
            archetype_id_iter: query_state.matched_archetype_ids.iter(),
            table_entities: &[],
            archetype_entities: &[],
            fetch_state,
            filter_state,
            fetch_state_vec,
            filter_state_vec,
            current_len: 0,
            current_row: 0,
            last_run,
            this_run,
            _marker: PhantomData,
        }
    }
}

impl<'w, 's, Q: QueryFetchGroup, F: QueryFetchGroup> Iterator for TermQueryIter<'w, 's, Q, F> {
    type Item = Q::Item<'w>;

    #[inline(always)]
    fn next(&mut self) -> Option<Q::Item<'w>> {
        // SAFETY: invariants guaranteed by caller in `Self::new`
        unsafe {
            let fetch_state = if Q::SIZE <= 16 {
                ThinSlicePtr::from(&mut self.fetch_state.assume_init_mut()[..])
            } else {
                ThinSlicePtr::from(&mut self.fetch_state_vec[..])
            };

            let filter_state = if F::SIZE <= 16 {
                ThinSlicePtr::from(&mut self.filter_state.assume_init_mut()[..])
            } else {
                ThinSlicePtr::from(&mut self.filter_state_vec[..])
            };
            if Q::DENSE && F::DENSE {
                loop {
                    // we are on the beginning of the query, or finished processing a table, so skip to the next
                    if self.current_row == self.current_len {
                        let table_id = self.table_id_iter.next()?;
                        let table = self.tables.get(*table_id).debug_checked_unwrap();
                        // SAFETY: `table` is from the world that `fetch/filter` were created for,
                        // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                        Q::set_term_tables(fetch_state, table);
                        F::set_term_tables(filter_state, table);

                        self.table_entities = table.entities();
                        self.current_len = table.entity_count();
                        self.current_row = 0;
                        continue;
                    }

                    let entity = *self.table_entities.get_unchecked(self.current_row);
                    let table_row = TableRow::new(self.current_row);

                    // SAFETY:
                    // - set_table was called prior.
                    // - `current_row` must be a table row in range of the current table,
                    //   because if it was not, then the if above would have been executed.
                    // - fetch is only called once for each `entity`.
                    self.current_row += 1;
                    if F::filter_terms(
                        self.world,
                        self.last_run,
                        self.this_run,
                        filter_state,
                        entity,
                        table_row,
                    ) {
                        return Some(Q::fetch_terms(
                            self.world,
                            self.last_run,
                            self.this_run,
                            fetch_state,
                            entity,
                            table_row,
                        ));
                    }
                }
            } else {
                loop {
                    if self.current_row == self.current_len {
                        let archetype_id = self.archetype_id_iter.next()?;
                        let archetype = self.archetypes.get(*archetype_id).debug_checked_unwrap();
                        // SAFETY: `archetype` and `tables` are from the world that `fetch/filter` were created for,
                        // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                        let table = self.tables.get(archetype.table_id()).debug_checked_unwrap();
                        Q::set_term_archetypes(fetch_state, archetype, table);
                        F::set_term_archetypes(filter_state, archetype, table);
                        self.archetype_entities = archetype.entities();
                        self.current_len = archetype.len();
                        self.current_row = 0;
                        continue;
                    }

                    let archetype_entity = self.archetype_entities.get_unchecked(self.current_row);

                    self.current_row += 1;
                    let entity = archetype_entity.entity();
                    let table_row = archetype_entity.table_row();
                    if F::filter_terms(
                        self.world,
                        self.last_run,
                        self.this_run,
                        filter_state,
                        entity,
                        table_row,
                    ) {
                        return Some(Q::fetch_terms(
                            self.world,
                            self.last_run,
                            self.this_run,
                            fetch_state,
                            entity,
                            table_row,
                        ));
                    }
                }
            }
        }
    }
}
