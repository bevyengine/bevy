use std::{alloc::Layout, marker::PhantomData, mem::MaybeUninit};

use fixedbitset::FixedBitSet;

use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::{ComponentId, Tick},
    entity::Entity,
    prelude::World,
    query::{Access, DebugCheckedUnwrap, FilteredAccess, QueryEntityError, QuerySingleError},
    storage::{Table, TableId, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, WorldId},
};

use super::{
    FetchedTerm, QueryTermGroup, Term, TermOperator, TermQueryIter, TermQueryIterUntyped, TermState,
};

// Used to avoid allocating space for fetched terms in the hot loop
// Instead we re-use a buffer we allocate when the query or iterator is created
pub(crate) struct FetchBuffer {
    mem: *mut u8,
    len: usize,
}

impl FetchBuffer {
    #[inline]
    pub(crate) fn new(len: usize) -> Self {
        Self {
            // SAFETY: FetchedTerm has non-zero size
            mem: unsafe { std::alloc::alloc(Layout::array::<FetchedTerm>(len).unwrap_unchecked()) },
            len,
        }
    }

    /// Access the [`FetchBuffer`] as a slice of [`FetchedTerm`]
    ///
    /// # Safety
    ///
    /// Caller must manually enforce they have unique access to [`FetchBuffer`]
    #[allow(clippy::mut_from_ref)]
    #[inline]
    pub(crate) unsafe fn as_uninit<'w>(&self) -> &mut [MaybeUninit<FetchedTerm<'w>>] {
        std::slice::from_raw_parts_mut(self.mem.cast(), self.len)
    }
}

impl Drop for FetchBuffer {
    fn drop(&mut self) {
        // SAFETY: len is the same length as when the data was allocated and mem is owned by FetchBuffer
        unsafe {
            std::alloc::dealloc(
                self.mem,
                Layout::array::<FetchedTerm>(self.len).unwrap_unchecked(),
            );
        }
    }
}

// SAFETY: FetchBuffer is just a pointer to memory
unsafe impl Send for FetchBuffer {}
// SAFETY: FetchBuffer is just a pointer to memory
unsafe impl Sync for FetchBuffer {}

/// Provides scoped access to a [`World`] state according to a given [`QueryTermGroup`]
pub struct TermQueryState<Q: QueryTermGroup = (), F: QueryTermGroup = ()> {
    world_id: WorldId,
    fetches: FetchBuffer,
    pub(crate) terms: Vec<Term>,
    pub(crate) archetype_generation: ArchetypeGeneration,
    pub(crate) matched_tables: FixedBitSet,
    pub(crate) matched_archetypes: FixedBitSet,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    pub(crate) component_access: FilteredAccess<ComponentId>,
    // NOTE: we maintain both a TableId bitset and a vec because iterating the vec is faster
    pub(crate) matched_table_ids: Vec<TableId>,
    // NOTE: we maintain both a ArchetypeId bitset and a vec because iterating the vec is faster
    pub(crate) matched_archetype_ids: Vec<ArchetypeId>,
    _marker: PhantomData<fn() -> (Q, F)>,
}

/// The read-only variant of the item type returned when a [`TermQueryState`] is iterated over immutably
pub type ROTermItem<'w, Q> = <<Q as QueryTermGroup>::ReadOnly as QueryTermGroup>::Item<'w>;

impl<Q: QueryTermGroup, F: QueryTermGroup> TermQueryState<Q, F> {
    /// Creates a new [`TermQueryState`] from a given [`World`] generating terms from `Q` and `F`.
    pub fn new(world: &mut World) -> Self {
        let mut terms = Vec::new();
        Q::init_terms(world, &mut terms);
        F::ReadOnly::init_terms(world, &mut terms);
        // SAFETY: We know these terms match Q as we generated them directly
        unsafe { Self::from_terms(world, terms) }
    }

    /// Creates a new [`TermQueryState`] from a given [`World`] and set of terms.
    ///
    /// # Safety
    ///
    /// Q must have the same or weaker access requirements than the given terms
    #[inline]
    pub unsafe fn from_terms(world: &mut World, terms: Vec<Term>) -> Self {
        let mut component_access = FilteredAccess::default();
        terms
            .iter()
            .for_each(|term| term.update_component_access(&mut component_access));
        let fetches = FetchBuffer::new(terms.len());

        Self {
            terms,
            fetches,
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
            matched_table_ids: Vec::new(),
            matched_archetype_ids: Vec::new(),
            archetype_component_access: Access::default(),
            component_access,
            matched_tables: FixedBitSet::default(),
            matched_archetypes: FixedBitSet::default(),
            _marker: PhantomData::default(),
        }
    }

    /// Update the current [`TermQueryState`] with information from the provided [`Archetype`]
    /// (if applicable, i.e. if the archetype has any intersecting [`ComponentId`] with the current [`TermQueryState`]).
    #[inline]
    pub fn new_archetype(&mut self, archetype: &Archetype) {
        if self.terms.iter().all(|t| {
            t.matches_component_set(&|id| {
                t.operator == TermOperator::Optional || archetype.contains(id)
            })
        }) {
            self.terms.iter().for_each(|t| {
                t.update_archetype_component_access(
                    archetype,
                    &mut self.archetype_component_access,
                );
            });

            let archetype_index = archetype.id().index();
            if !self.matched_archetypes.contains(archetype_index) {
                self.matched_archetypes.grow(archetype_index + 1);
                self.matched_archetypes.set(archetype_index, true);
                self.matched_archetype_ids.push(archetype.id());
            }
            let table_index = archetype.table_id().index();
            if !self.matched_tables.contains(table_index) {
                self.matched_tables.grow(table_index + 1);
                self.matched_tables.set(table_index, true);
                self.matched_table_ids.push(archetype.table_id());
            }
        }
    }

    #[inline]
    pub(crate) unsafe fn init_term_state<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Vec<TermState<'w>> {
        self.terms
            .iter()
            .map(|term| term.init_state(world, last_run, this_run))
            .collect()
    }

    /// Re-interpret this [`TermQueryState`] as it's read only counterpart
    ///
    /// Note: This doesn't change any of the underlying [`Term`]s
    pub fn as_readonly(&self) -> &TermQueryState<Q::ReadOnly> {
        // SAFETY: ReadOnly versions of a query have a subset of the access requirements
        unsafe { std::mem::transmute(self) }
    }

    /// Re-interpret this [`TermQueryState`] as one without the associated filter type parameter
    pub fn filterless(&self) -> &TermQueryState<Q> {
        // SAFETY: The filter type parameter isn't used after construction of the state so dropping it is a nop
        unsafe { std::mem::transmute(self) }
    }

    /// Returns true if this query state could be iterate as type `NewQ`
    pub fn interpretable_as<NewQ: QueryTermGroup>(&self, world: &mut World) -> bool {
        let mut terms = Vec::new();
        NewQ::init_terms(world, &mut terms);
        terms
            .iter()
            .enumerate()
            .all(|(i, a)| self.terms.get(i).is_some_and(|b| b.interpretable_as(a)))
    }

    /// Converts this [`TermQueryState`] to another compatible [`TermQueryState`].
    ///
    /// Consider using [`TermQueryState::as_readonly`] or [`TermQueryState::filterless`] instead
    /// where possible.
    pub fn try_transmute<NewQ: QueryTermGroup>(
        self,
        world: &mut World,
    ) -> Option<TermQueryState<NewQ>> {
        if self.interpretable_as::<NewQ>(world) {
            // SAFETY: Just checked that the type is compatible
            Some(unsafe { std::mem::transmute(self) })
        } else {
            None
        }
    }

    /// Converts this [`TermQueryState`] to any other [`TermQueryState`].
    ///
    /// Consider using [`TermQueryState::as_readonly`] or [`TermQueryState::filterless`] instead
    /// which are safe functions.
    ///
    /// # Safety
    ///
    /// `NewQ` must have a subset of the access that `Q` does and match the exact same archetypes/tables
    pub unsafe fn transmute<NewQ: QueryTermGroup>(self) -> TermQueryState<NewQ> {
        std::mem::transmute(self)
    }

    /// Converts this [`TermQueryState`] reference to any other [`TermQueryState`].
    ///
    /// Consider using [`TermQueryState::as_readonly`] or [`TermQueryState::filterless`] instead
    /// which are safe functions.
    ///
    /// # Safety
    ///
    /// `NewQ` must have a subset of the access that `Q` does and match the exact same archetypes/tables
    pub unsafe fn transmute_ref<NewQ: QueryTermGroup>(&self) -> &TermQueryState<NewQ> {
        std::mem::transmute(self)
    }

    /// Converts this [`TermQueryState`] reference to any other [`TermQueryState`].
    ///
    /// Consider using [`TermQueryState::as_readonly`] or [`TermQueryState::filterless`] instead
    /// which are safe functions.
    ///
    /// # Safety
    ///
    /// `NewQ` must have a subset of the access that `Q` does and match the exact same archetypes/tables
    pub unsafe fn transmute_mut<NewQ: QueryTermGroup>(&mut self) -> &mut TermQueryState<NewQ> {
        std::mem::transmute(self)
    }

    /// # Panics
    ///
    /// If `world_id` does not match the [`World`] used to call `QueryState::new` for this instance.
    ///
    /// Many unsafe query methods require the world to match for soundness. This function is the easiest
    /// way of ensuring that it matches.
    #[inline]
    pub fn validate_world(&self, world_id: WorldId) {
        assert!(
            world_id == self.world_id,
            "Attempted to use {} with a mismatched World. TermQueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>(),
        );
    }

    /// Updates the state's internal view of the [`World`]'s archetypes. If this is not called before querying data,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if a `manual` method (such as [`Self::get_manual`]) is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// non-`manual` methods such as [`TermQueryState::get`] do not need to call this as it will be automatically called for them.
    ///
    /// If you have an [`UnsafeWorldCell`] instead of `&World`, consider using [`TermQueryState::update_archetypes_unsafe_world_cell`].
    ///
    /// # Panics
    ///
    /// If `world` does not match the one used to call `QueryState::new` for this instance.
    #[inline]
    pub fn update_archetypes(&mut self, world: &World) {
        self.update_archetypes_unsafe_world_cell(world.as_unsafe_world_cell_readonly());
    }

    /// Updates the state's internal view of the `world`'s archetypes. If this is not called before querying data,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if a `manual` method (such as [`Self::get_manual`]) is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// non-`manual` methods such as [`TermQueryState::get`] do not need to call this as it will be automatically called for them.
    ///
    /// # Note
    ///
    /// This method only accesses world metadata.
    ///
    /// # Panics
    ///
    /// If `world` does not match the one used to call `TermQueryState::new` for this instance.
    #[inline]
    pub fn update_archetypes_unsafe_world_cell(&mut self, world: UnsafeWorldCell) {
        self.validate_world(world.id());
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            self.new_archetype(&archetypes[ArchetypeId::new(archetype_index)]);
        }
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`Self::iter_mut`] for write-queries.
    #[inline]
    pub fn iter<'w, 's>(&'s mut self, world: &'w World) -> TermQueryIter<'w, 's, Q::ReadOnly> {
        self.update_archetypes(world);
        // SAFETY: query is read only
        unsafe {
            self.as_readonly().iter_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> TermQueryIter<'w, 's, Q> {
        self.update_archetypes(world);
        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: query has unique world access
        unsafe {
            self.iter_unchecked_manual(world.as_unsafe_world_cell(), last_change_tick, change_tick)
        }
    }

    /// Returns an [`Iterator`] for the given [`World`], where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    #[inline]
    pub(crate) unsafe fn iter_unchecked_manual<'w, 's>(
        &'s self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> TermQueryIter<'w, 's, Q> {
        TermQueryIter::new(world, self.filterless(), last_run, this_run)
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    #[inline]
    pub fn iter_raw<'w, 's>(&'s mut self, world: &'w mut World) -> TermQueryIterUntyped<'w, 's> {
        self.update_archetypes(world);
        let last_run = world.last_change_tick();
        let this_run = world.change_tick();
        // SAFETY: query has unique world access
        unsafe { self.iter_raw_manual(world.as_unsafe_world_cell(), last_run, this_run) }
    }

    /// Returns an [`Iterator`] for the given [`World`], where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    pub(crate) unsafe fn iter_raw_manual<'w, 's>(
        &'s self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> TermQueryIterUntyped<'w, 's> {
        TermQueryIterUntyped::new(world, self.filterless(), last_run, this_run)
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for write-queries.
    #[inline]
    pub fn get<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROTermItem<'w, Q>, QueryEntityError> {
        self.update_archetypes(world);
        // SAFETY: query is read only
        unsafe {
            self.as_readonly().get_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                entity,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    #[inline]
    pub fn get_mut<'w>(
        &mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Result<Q::Item<'w>, QueryEntityError> {
        self.update_archetypes(world);
        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: query has unique world access
        unsafe {
            self.get_unchecked_manual(
                world.as_unsafe_world_cell(),
                entity,
                last_change_tick,
                change_tick,
            )
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This method is slightly more efficient than [`TermQueryState::get`] in some situations, since
    /// it does not update this instance's internal cache. This method will return an error if `entity`
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`TermQueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`TermQueryState::new`], `TermQueryState::get`, or any method with mutable
    /// access to `self`.
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for mutable queries.
    #[inline]
    pub fn get_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROTermItem<'w, Q>, QueryEntityError> {
        self.validate_world(world.id());
        // SAFETY: query is read only and world is validated
        unsafe {
            self.as_readonly().get_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                entity,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn get_unchecked<'w>(
        &mut self,
        world: UnsafeWorldCell<'w>,
        entity: Entity,
    ) -> Result<Q::Item<'w>, QueryEntityError> {
        self.update_archetypes_unsafe_world_cell(world);
        self.get_unchecked_manual(world, entity, world.last_change_tick(), world.change_tick())
    }

    /// Gets the query result for the given [`World`] and [`Entity`], where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    ///
    /// This must be called on the same `World` that the `TermQuery` was generated from:
    /// use `TermQueryState::validate_world` to verify this.
    #[inline]
    pub(crate) unsafe fn get_unchecked_manual<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        entity: Entity,
        last_run: Tick,
        this_run: Tick,
    ) -> Result<Q::Item<'w>, QueryEntityError> {
        let location = world
            .entities()
            .get(entity)
            .ok_or(QueryEntityError::NoSuchEntity(entity))?;
        if !self
            .matched_archetypes
            .contains(location.archetype_id.index())
        {
            return Err(QueryEntityError::QueryDoesNotMatch(entity));
        }
        let mut term_state = self.init_term_state(world, last_run, this_run);

        let table = world
            .storages()
            .tables
            .get(location.table_id)
            .debug_checked_unwrap();

        self.set_table(&mut term_state, table);
        let fetches = self.fetch(
            &term_state,
            entity,
            location.table_row,
            self.fetches.as_uninit(),
        );
        Ok(Q::from_fetches(&mut fetches.iter()))
    }

    /// Returns a single immutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// This can only be called for read-only queries,
    /// see [`single_mut`](Self::single_mut) for write-queries.
    ///
    /// # Panics
    ///
    /// Panics if the number of query results is not exactly one. Use
    /// [`get_single`](Self::get_single) to return a `Result` instead of panicking.
    #[track_caller]
    #[inline]
    pub fn single<'w>(&mut self, world: &'w World) -> ROTermItem<'w, Q> {
        self.get_single(world).unwrap()
    }

    /// Returns a single immutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// This can only be called for read-only queries,
    /// see [`get_single_mut`](Self::get_single_mut) for write-queries.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    #[inline]
    pub fn get_single<'w>(
        &mut self,
        world: &'w World,
    ) -> Result<ROTermItem<'w, Q>, QuerySingleError> {
        self.update_archetypes(world);

        // SAFETY: query is read only
        unsafe {
            self.as_readonly().get_single_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns a single mutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// # Panics
    ///
    /// Panics if the number of query results is not exactly one. Use
    /// [`get_single_mut`](Self::get_single_mut) to return a `Result` instead of panicking.
    #[track_caller]
    #[inline]
    pub fn single_mut<'w>(&mut self, world: &'w mut World) -> Q::Item<'w> {
        // SAFETY: query has unique world access
        self.get_single_mut(world).unwrap()
    }

    /// Returns a single mutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    #[inline]
    pub fn get_single_mut<'w>(
        &mut self,
        world: &'w mut World,
    ) -> Result<Q::Item<'w>, QuerySingleError> {
        self.update_archetypes(world);

        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: query has unique world access
        unsafe {
            self.get_single_unchecked_manual(
                world.as_unsafe_world_cell(),
                last_change_tick,
                change_tick,
            )
        }
    }

    /// Returns a query result when there is exactly one entity matching the query.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn get_single_unchecked<'w>(
        &mut self,
        world: UnsafeWorldCell<'w>,
    ) -> Result<Q::Item<'w>, QuerySingleError> {
        self.update_archetypes_unsafe_world_cell(world);
        self.get_single_unchecked_manual(world, world.last_change_tick(), world.change_tick())
    }

    /// Returns a query result when there is exactly one entity matching the query,
    /// where the last change and the current change tick are given.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn get_single_unchecked_manual<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Result<Q::Item<'w>, QuerySingleError> {
        let mut query = self.iter_unchecked_manual(world, last_run, this_run);
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(QuerySingleError::NoEntities(std::any::type_name::<Self>())),
            (Some(_), _) => Err(QuerySingleError::MultipleEntities(std::any::type_name::<
                Self,
            >())),
        }
    }

    /// Returns a single set of fetched query terms when there is exactly one entity matching
    /// the query.
    ///
    /// # Panics
    ///
    /// Panics if the number of query results is not exactly one. Use
    /// [`get_single_raw`](Self::get_single_raw) to return a `Result` instead of panicking.
    #[inline]
    pub fn single_raw<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> (&'s Vec<Term>, Vec<FetchedTerm<'w>>) {
        self.get_single_raw(world).unwrap()
    }

    /// Returns a single set of fetched query terms when there is exactly one entity matching
    /// the query.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    #[inline]
    pub fn get_single_raw<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> Result<(&'s Vec<Term>, Vec<FetchedTerm<'w>>), QuerySingleError> {
        let mut query = self.iter_raw(world);
        let first = query.next();
        let extra = query.next().is_some();

        match (first, extra) {
            (Some(r), false) => Ok(r),
            (None, _) => Err(QuerySingleError::NoEntities(std::any::type_name::<Self>())),
            (Some(_), _) => Err(QuerySingleError::MultipleEntities(std::any::type_name::<
                Self,
            >())),
        }
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// This can only be called for read-only queries, see [`Self::for_each_mut`] for write-queries.
    #[inline]
    pub fn for_each<'w, FN: FnMut(ROTermItem<'w, Q>)>(&mut self, world: &'w World, func: FN) {
        self.update_archetypes(world);
        // SAFETY: query is read only
        unsafe {
            self.as_readonly().for_each_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                func,
                world.last_change_tick(),
                world.read_change_tick(),
            );
        }
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// `iter_mut()` method, but cannot be chained like a normal [`Iterator`].
    #[inline]
    pub fn for_each_mut<'w, FN: FnMut(Q::Item<'w>)>(&mut self, world: &'w mut World, func: FN) {
        self.update_archetypes(world);
        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: query has unique world access
        unsafe {
            self.for_each_unchecked_manual(
                world.as_unsafe_world_cell(),
                func,
                last_change_tick,
                change_tick,
            );
        }
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn for_each_unchecked<'w, FN: FnMut(Q::Item<'w>)>(
        &mut self,
        world: UnsafeWorldCell<'w>,
        func: FN,
    ) {
        self.update_archetypes_unsafe_world_cell(world);
        self.for_each_unchecked_manual(world, func, world.last_change_tick(), world.change_tick());
    }

    /// Runs `func` on each query result for the given [`World`], where the last change and
    /// the current change tick are given. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    pub(crate) unsafe fn for_each_unchecked_manual<'w, FN: FnMut(Q::Item<'w>)>(
        &self,
        world: UnsafeWorldCell<'w>,
        mut func: FN,
        last_run: Tick,
        this_run: Tick,
    ) {
        let mut term_state = self.init_term_state(world, last_run, this_run);
        let dense = term_state.iter().all(|t| t.dense());
        let fetch_buffer = FetchBuffer::new(self.terms.len());

        let tables = &world.storages().tables;
        if dense {
            for table_id in &self.matched_table_ids {
                let table = tables.get(*table_id).debug_checked_unwrap();
                self.set_table(&mut term_state, table);

                let entities = table.entities();
                for row in 0..table.entity_count() {
                    let entity = *entities.get_unchecked(row);
                    let row = TableRow::new(row);
                    if !self.filter_fetch(&term_state, entity, row) {
                        continue;
                    }
                    let fetches = self.fetch(&term_state, entity, row, fetch_buffer.as_uninit());
                    func(Q::from_fetches(&mut fetches.iter()));
                }
            }
        } else {
            let archetypes = world.archetypes();
            for archetype_id in &self.matched_archetype_ids {
                let archetype = archetypes.get(*archetype_id).debug_checked_unwrap();
                let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                self.set_archetype(&mut term_state, archetype, table);

                let entities = archetype.entities();
                for idx in 0..archetype.len() {
                    let archetype_entity = entities.get_unchecked(idx);
                    let entity = archetype_entity.entity();
                    let row = archetype_entity.table_row();
                    if !self.filter_fetch(&term_state, entity, row) {
                        continue;
                    }

                    let fetches = self.fetch(&term_state, entity, row, fetch_buffer.as_uninit());
                    func(Q::from_fetches(&mut fetches.iter()));
                }
            }
        }
    }

    // Updates the internal state for each term by calling [`Term::set_archetype`] on each term
    #[inline]
    pub(crate) unsafe fn set_archetype<'w>(
        &self,
        state: &mut [TermState<'w>],
        archetype: &'w Archetype,
        table: &'w Table,
    ) {
        let len = self.terms.len();
        let terms = &self.terms[..len];
        let state = &mut state[..len];

        for i in 0..len {
            let term = &terms[i];
            let state = &mut state[i];
            term.set_archetype(state, archetype, table);
        }
    }

    // Updates the internal state for each term by calling [`Term::setset_table_archetype`] on each term
    #[inline]
    pub(crate) unsafe fn set_table<'w>(&self, state: &mut [TermState<'w>], table: &'w Table) {
        let len = self.terms.len();
        let terms = &self.terms[..len];
        let state = &mut state[..len];

        for i in 0..len {
            let term = &terms[i];
            let state = &mut state[i];
            term.set_table(state, table);
        }
    }

    // Resolves this query against the given entity and table row, returns true if the entity matches
    #[inline(always)]
    pub(crate) unsafe fn filter_fetch(
        &self,
        state: &[TermState<'_>],
        entity: Entity,
        table_row: TableRow,
    ) -> bool {
        let len = self.terms.len();
        let terms = &self.terms[..len];
        let state = &state[..len];

        for i in 0..len {
            let term = &terms[i];
            let state = &state[i];
            if !term.filter_fetch(state, entity, table_row) {
                return false;
            }
        }

        true
    }

    /// Resolves this query against the given entity and table row, returns the fetched slice of [`FetchedTerm`]
    ///
    /// # Safety
    ///
    /// Caller must ensure that `mem` has room for the terms that will be fetched by `self.terms` and `state`
    /// is the same state created in `init_term_state`
    #[inline(always)]
    pub(crate) unsafe fn fetch<'w, 's, 'f>(
        &'s self,
        state: &'s [TermState<'w>],
        entity: Entity,
        table_row: TableRow,
        mem: &'f mut [MaybeUninit<FetchedTerm<'w>>],
    ) -> &'f mut [FetchedTerm<'w>] {
        let len = self.terms.len();
        let terms = &self.terms[..len];
        let state = &state[..len];

        for i in 0..len {
            let term = &terms[i];
            let state = &state[i];
            mem[i].write(term.fetch(state, entity, table_row));
        }

        std::mem::transmute(mem)
    }
}
