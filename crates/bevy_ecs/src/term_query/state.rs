use std::marker::PhantomData;

use fixedbitset::FixedBitSet;

use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::{ComponentId, Tick},
    entity::Entity,
    prelude::World,
    query::{Access, DebugCheckedUnwrap, FilteredAccess, QueryEntityError, QuerySingleError},
    storage::TableId,
    world::{unsafe_world_cell::UnsafeWorldCell, WorldId},
};

use super::{
    Fetchable, FetchedTerm, QueryTermGroup, Term, TermQueryIter, TermQueryIterUntyped, TermState,
};

pub struct TermQueryState<Q: QueryTermGroup = (), F: QueryTermGroup = ()> {
    world_id: WorldId,
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

pub type ROTermItem<'w, Q> = <<Q as QueryTermGroup>::ReadOnly as QueryTermGroup>::Item<'w>;

impl<Q: QueryTermGroup, F: QueryTermGroup> TermQueryState<Q, F> {
    pub fn new(world: &mut World) -> Self {
        let mut terms = Vec::new();
        Q::init_terms(world, &mut terms);
        F::ReadOnly::init_terms(world, &mut terms);
        Self::from_terms(world, terms)
    }

    pub fn new_archetype(&mut self, archetype: &Archetype) {
        if self
            .terms
            .iter()
            .all(|t| t.matches_component_set(&|id| archetype.contains(id)))
        {
            self.terms.iter().for_each(|t| {
                t.update_archetype_component_access(archetype, &mut self.archetype_component_access)
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

    pub unsafe fn init_term_state<'w>(
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

    pub fn from_terms(world: &mut World, terms: Vec<Term>) -> Self {
        let mut component_access = FilteredAccess::default();
        terms
            .iter()
            .for_each(|term| term.update_component_access(&mut component_access));
        Self {
            terms,
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

    pub unsafe fn as_readonly(&self) -> &TermQueryState<Q::ReadOnly> {
        // TODO: Checked version
        std::mem::transmute(self)
    }

    pub unsafe fn transmute<O: QueryTermGroup>(self) -> TermQueryState<O> {
        std::mem::transmute(self)
    }

    pub unsafe fn transmute_ref<O: QueryTermGroup>(&self) -> &TermQueryState<O> {
        std::mem::transmute(self)
    }

    pub unsafe fn transmute_mut<O: QueryTermGroup>(&mut self) -> &mut TermQueryState<O> {
        std::mem::transmute(self)
    }

    pub fn filterless(&self) -> &TermQueryState<Q> {
        unsafe { std::mem::transmute(self) }
    }

    pub fn validate_world(&self, world_id: WorldId) {
        assert!(
            world_id == self.world_id,
            "Attempted to use {} with a mismatched World. TermQueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>(),
        );
    }

    #[inline]
    pub fn update_archetypes(&mut self, world: &World) {
        self.update_archetypes_unsafe_world_cell(world.as_unsafe_world_cell_readonly());
    }

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
        let last_run = world.change_tick();
        let this_run = world.last_change_tick();

        unsafe {
            TermQueryIterUntyped::new(
                world.as_unsafe_world_cell(),
                self.filterless(),
                last_run,
                this_run,
            )
        }
    }

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

        let mut fetched_terms =
            self.terms
                .iter()
                .zip(term_state.iter_mut())
                .map(|(term, state)| {
                    term.set_table(state, table);
                    term.fetch(state, entity, location.table_row)
                });
        Ok(Q::from_fetches(&mut fetched_terms))
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

    #[inline]
    pub fn single_raw<'w>(&mut self, world: &'w mut World) -> Vec<FetchedTerm<'w>> {
        self.get_single_raw(world).unwrap()
    }

    #[inline]
    pub fn get_single_raw<'w>(
        &mut self,
        world: &'w mut World,
    ) -> Result<Vec<FetchedTerm<'w>>, QuerySingleError> {
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
}
