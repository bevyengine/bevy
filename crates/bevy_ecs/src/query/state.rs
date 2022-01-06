use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::ComponentId,
    entity::Entity,
    query::{
        Access, Fetch, FetchState, FilterFetch, FilteredAccess, NopFetch, QueryCombinationIter,
        QueryIter, WorldQuery,
    },
    storage::TableId,
    world::{World, WorldId},
};
use bevy_tasks::TaskPool;
use fixedbitset::FixedBitSet;
use thiserror::Error;

/// Provides scoped access to a [`World`] state according to a given [`WorldQuery`] and query filter.
pub struct QueryState<Q: WorldQuery, F: WorldQuery = ()>
where
    F::Fetch: FilterFetch,
{
    world_id: WorldId,
    pub(crate) archetype_generation: ArchetypeGeneration,
    pub(crate) matched_tables: FixedBitSet,
    pub(crate) matched_archetypes: FixedBitSet,
    pub(crate) archetype_component_access: Access<ArchetypeComponentId>,
    pub(crate) component_access: FilteredAccess<ComponentId>,
    // NOTE: we maintain both a TableId bitset and a vec because iterating the vec is faster
    pub(crate) matched_table_ids: Vec<TableId>,
    // NOTE: we maintain both a ArchetypeId bitset and a vec because iterating the vec is faster
    pub(crate) matched_archetype_ids: Vec<ArchetypeId>,
    pub(crate) fetch_state: Q::State,
    pub(crate) filter_state: F::State,
}

impl<Q: WorldQuery, F: WorldQuery> QueryState<Q, F>
where
    F::Fetch: FilterFetch,
{
    /// Creates a new [`QueryState`] from a given [`World`] and inherits the result of `world.id()`.
    pub fn new(world: &mut World) -> Self {
        let fetch_state = <Q::State as FetchState>::init(world);
        let filter_state = <F::State as FetchState>::init(world);

        let mut component_access = FilteredAccess::default();
        fetch_state.update_component_access(&mut component_access);

        // Use a temporary empty FilteredAccess for filters. This prevents them from conflicting with the
        // main Query's `fetch_state` access. Filters are allowed to conflict with the main query fetch
        // because they are evaluated *before* a specific reference is constructed.
        let mut filter_component_access = FilteredAccess::default();
        filter_state.update_component_access(&mut filter_component_access);

        // Merge the temporary filter access with the main access. This ensures that filter access is
        // properly considered in a global "cross-query" context (both within systems and across systems).
        component_access.extend(&filter_component_access);

        let mut state = Self {
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
            matched_table_ids: Vec::new(),
            matched_archetype_ids: Vec::new(),
            fetch_state,
            filter_state,
            component_access,
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            archetype_component_access: Default::default(),
        };
        state.update_archetypes(world);
        state
    }

    /// Checks if the query is empty for the given [`World`], where the last change and current tick are given.
    #[inline]
    pub fn is_empty(&self, world: &World, last_change_tick: u32, change_tick: u32) -> bool {
        // SAFE: NopFetch does not access any members while &self ensures no one has exclusive access
        unsafe {
            self.iter_unchecked_manual::<NopFetch<Q::State>>(world, last_change_tick, change_tick)
                .next()
                .is_none()
        }
    }

    /// Takes a query for the given [`World`], checks if the given world is the same as the query, and
    /// generates new archetypes for the given world.
    ///
    /// # Panics
    ///
    /// Panics if the `world.id()` does not equal the current [`QueryState`] internal id.
    pub fn update_archetypes(&mut self, world: &World) {
        self.validate_world(world);
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            self.new_archetype(&archetypes[ArchetypeId::new(archetype_index)]);
        }
    }

    #[inline]
    pub fn validate_world(&self, world: &World) {
        if world.id() != self.world_id {
            panic!("Attempted to use {} with a mismatched World. QueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>());
        }
    }

    /// Creates a new [`Archetype`].
    pub fn new_archetype(&mut self, archetype: &Archetype) {
        if self.fetch_state.matches_archetype(archetype)
            && self.filter_state.matches_archetype(archetype)
        {
            self.fetch_state
                .update_archetype_component_access(archetype, &mut self.archetype_component_access);
            self.filter_state
                .update_archetype_component_access(archetype, &mut self.archetype_component_access);
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

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for write-queries.
    #[inline]
    pub fn get<'w, 's>(
        &'s mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item, QueryEntityError> {
        self.update_archetypes(world);
        // SAFETY: query is read only
        unsafe {
            self.get_unchecked_manual::<Q::ReadOnlyFetch>(
                world,
                entity,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    #[inline]
    pub fn get_mut<'w, 's>(
        &'s mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w, 's>>::Item, QueryEntityError> {
        self.update_archetypes(world);
        // SAFETY: query has unique world access
        unsafe {
            self.get_unchecked_manual::<Q::Fetch>(
                world,
                entity,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    #[inline]
    pub fn get_manual<'w, 's>(
        &'s self,
        world: &'w World,
        entity: Entity,
    ) -> Result<<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item, QueryEntityError> {
        self.validate_world(world);
        // SAFETY: query is read only and world is validated
        unsafe {
            self.get_unchecked_manual::<Q::ReadOnlyFetch>(
                world,
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
    pub unsafe fn get_unchecked<'w, 's>(
        &'s mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w, 's>>::Item, QueryEntityError> {
        self.update_archetypes(world);
        self.get_unchecked_manual::<Q::Fetch>(
            world,
            entity,
            world.last_change_tick(),
            world.read_change_tick(),
        )
    }

    /// Gets the query result for the given [`World`] and [`Entity`], where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub(crate) unsafe fn get_unchecked_manual<'w, 's, QF: Fetch<'w, 's, State = Q::State>>(
        &'s self,
        world: &'w World,
        entity: Entity,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Result<QF::Item, QueryEntityError> {
        let location = world
            .entities
            .get(entity)
            .ok_or(QueryEntityError::NoSuchEntity)?;
        if !self
            .matched_archetypes
            .contains(location.archetype_id.index())
        {
            return Err(QueryEntityError::QueryDoesNotMatch);
        }
        let archetype = &world.archetypes[location.archetype_id];
        let mut fetch = QF::init(world, &self.fetch_state, last_change_tick, change_tick);
        let mut filter =
            <F::Fetch as Fetch>::init(world, &self.filter_state, last_change_tick, change_tick);

        fetch.set_archetype(&self.fetch_state, archetype, &world.storages().tables);
        filter.set_archetype(&self.filter_state, archetype, &world.storages().tables);
        if filter.archetype_filter_fetch(location.index) {
            Ok(fetch.archetype_fetch(location.index))
        } else {
            Err(QueryEntityError::QueryDoesNotMatch)
        }
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`Self::iter_mut`] for write-queries.
    #[inline]
    pub fn iter<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryIter<'w, 's, Q, Q::ReadOnlyFetch, F> {
        // SAFETY: query is read only
        unsafe {
            self.update_archetypes(world);
            self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
        }
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    #[inline]
    pub fn iter_mut<'w, 's>(
        &'s mut self,
        world: &'w mut World,
    ) -> QueryIter<'w, 's, Q, Q::Fetch, F> {
        // SAFETY: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
        }
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    /// This can only be called for read-only queries.
    ///
    ///  For permutations of size K of query returning N results, you will get:
    /// - if K == N: one permutation of all query results
    /// - if K < N: all possible K-sized combinations of query results, without repetition
    /// - if K > N: empty set (no K-sized combinations exist)
    ///
    /// This can only be called for read-only queries, see [`Self::iter_combinations_mut`] for
    /// write-queries.
    #[inline]
    pub fn iter_manual<'w, 's>(
        &'s self,
        world: &'w World,
    ) -> QueryIter<'w, 's, Q, Q::ReadOnlyFetch, F> {
        self.validate_world(world);
        // SAFETY: query is read only and world is validated
        unsafe {
            self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
        }
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    /// This can only be called for read-only queries.
    ///
    ///  For permutations of size K of query returning N results, you will get:
    /// - if K == N: one permutation of all query results
    /// - if K < N: all possible K-sized combinations of query results, without repetition
    /// - if K > N: empty set (no K-sized combinations exist)
    ///
    /// This can only be called for read-only queries, see [`Self::iter_combinations_mut`] for
    /// write-queries.
    #[inline]
    pub fn iter_combinations<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w World,
    ) -> QueryCombinationIter<'w, 's, Q, Q::ReadOnlyFetch, F, K> {
        // SAFE: query is read only
        unsafe {
            self.update_archetypes(world);
            self.iter_combinations_unchecked_manual(
                world,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Iterates over all possible combinations of `K` query results for the given [`World`]
    /// without repetition.
    ///
    ///  For permutations of size K of query returning N results, you will get:
    /// - if K == N: one permutation of all query results
    /// - if K < N: all possible K-sized combinations of query results, without repetition
    /// - if K > N: empty set (no K-sized combinations exist)
    #[inline]
    pub fn iter_combinations_mut<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w mut World,
    ) -> QueryCombinationIter<'w, 's, Q, Q::Fetch, F, K> {
        // SAFE: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.iter_combinations_unchecked_manual(
                world,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_unchecked<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryIter<'w, 's, Q, Q::Fetch, F> {
        self.update_archetypes(world);
        self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results for the
    /// given [`World`] without repetition.
    /// This can only be called for read-only queries.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_combinations_unchecked<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w World,
    ) -> QueryCombinationIter<'w, 's, Q, Q::Fetch, F, K> {
        self.update_archetypes(world);
        self.iter_combinations_unchecked_manual(
            world,
            world.last_change_tick(),
            world.read_change_tick(),
        )
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
    pub(crate) unsafe fn iter_unchecked_manual<'w, 's, QF: Fetch<'w, 's, State = Q::State>>(
        &'s self,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryIter<'w, 's, Q, QF, F> {
        QueryIter::new(world, self, last_change_tick, change_tick)
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results for the
    /// given [`World`] without repetition.
    /// This can only be called for read-only queries.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    #[inline]
    pub(crate) unsafe fn iter_combinations_unchecked_manual<
        'w,
        's,
        QF: Fetch<'w, 's, State = Q::State>,
        const K: usize,
    >(
        &'s self,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryCombinationIter<'w, 's, Q, QF, F, K> {
        QueryCombinationIter::new(world, self, last_change_tick, change_tick)
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// This can only be called for read-only queries, see [`Self::for_each_mut`] for write-queries.
    #[inline]
    pub fn for_each<'w, 's, FN: FnMut(<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item)>(
        &'s mut self,
        world: &'w World,
        func: FN,
    ) {
        // SAFETY: query is read only
        unsafe {
            self.update_archetypes(world);
            self.for_each_unchecked_manual::<Q::ReadOnlyFetch, FN>(
                world,
                func,
                world.last_change_tick(),
                world.read_change_tick(),
            );
        }
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// `iter_mut()` method, but cannot be chained like a normal [`Iterator`].
    #[inline]
    pub fn for_each_mut<'w, 's, FN: FnMut(<Q::Fetch as Fetch<'w, 's>>::Item)>(
        &'s mut self,
        world: &'w mut World,
        func: FN,
    ) {
        // SAFETY: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.for_each_unchecked_manual::<Q::Fetch, FN>(
                world,
                func,
                world.last_change_tick(),
                world.read_change_tick(),
            );
        }
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// This can only be called for read-only queries.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn for_each_unchecked<'w, 's, FN: FnMut(<Q::Fetch as Fetch<'w, 's>>::Item)>(
        &'s mut self,
        world: &'w World,
        func: FN,
    ) {
        self.update_archetypes(world);
        self.for_each_unchecked_manual::<Q::Fetch, FN>(
            world,
            func,
            world.last_change_tick(),
            world.read_change_tick(),
        );
    }

    /// Runs `func` on each query result in parallel using the given `task_pool`.
    ///
    /// This can only be called for read-only queries, see [`Self::par_for_each_mut`] for
    /// write-queries.
    #[inline]
    pub fn par_for_each<
        'w,
        's,
        FN: Fn(<Q::ReadOnlyFetch as Fetch<'w, 's>>::Item) + Send + Sync + Clone,
    >(
        &'s mut self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: FN,
    ) {
        // SAFETY: query is read only
        unsafe {
            self.update_archetypes(world);
            self.par_for_each_unchecked_manual::<Q::ReadOnlyFetch, FN>(
                world,
                task_pool,
                batch_size,
                func,
                world.last_change_tick(),
                world.read_change_tick(),
            );
        }
    }

    /// Runs `func` on each query result in parallel using the given `task_pool`.
    #[inline]
    pub fn par_for_each_mut<
        'w,
        's,
        FN: Fn(<Q::Fetch as Fetch<'w, 's>>::Item) + Send + Sync + Clone,
    >(
        &'s mut self,
        world: &'w mut World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: FN,
    ) {
        // SAFETY: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.par_for_each_unchecked_manual::<Q::Fetch, FN>(
                world,
                task_pool,
                batch_size,
                func,
                world.last_change_tick(),
                world.read_change_tick(),
            );
        }
    }

    /// Runs `func` on each query result in parallel using the given `task_pool`.
    ///
    /// This can only be called for read-only queries.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn par_for_each_unchecked<
        'w,
        's,
        FN: Fn(<Q::Fetch as Fetch<'w, 's>>::Item) + Send + Sync + Clone,
    >(
        &'s mut self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: FN,
    ) {
        self.update_archetypes(world);
        self.par_for_each_unchecked_manual::<Q::Fetch, FN>(
            world,
            task_pool,
            batch_size,
            func,
            world.last_change_tick(),
            world.read_change_tick(),
        );
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
    pub(crate) unsafe fn for_each_unchecked_manual<
        'w,
        's,
        QF: Fetch<'w, 's, State = Q::State>,
        FN: FnMut(QF::Item),
    >(
        &'s self,
        world: &'w World,
        mut func: FN,
        last_change_tick: u32,
        change_tick: u32,
    ) {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        let mut fetch = QF::init(world, &self.fetch_state, last_change_tick, change_tick);
        let mut filter =
            <F::Fetch as Fetch>::init(world, &self.filter_state, last_change_tick, change_tick);
        if Q::Fetch::IS_DENSE && F::Fetch::IS_DENSE {
            let tables = &world.storages().tables;
            for table_id in self.matched_table_ids.iter() {
                let table = &tables[*table_id];
                fetch.set_table(&self.fetch_state, table);
                filter.set_table(&self.filter_state, table);

                for table_index in 0..table.len() {
                    if !filter.table_filter_fetch(table_index) {
                        continue;
                    }
                    let item = fetch.table_fetch(table_index);
                    func(item);
                }
            }
        } else {
            let archetypes = &world.archetypes;
            let tables = &world.storages().tables;
            for archetype_id in self.matched_archetype_ids.iter() {
                let archetype = &archetypes[*archetype_id];
                fetch.set_archetype(&self.fetch_state, archetype, tables);
                filter.set_archetype(&self.filter_state, archetype, tables);

                for archetype_index in 0..archetype.len() {
                    if !filter.archetype_filter_fetch(archetype_index) {
                        continue;
                    }
                    func(fetch.archetype_fetch(archetype_index));
                }
            }
        }
    }

    /// Runs `func` on each query result in parallel for the given [`World`], where the last change and
    /// the current change tick are given. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    pub(crate) unsafe fn par_for_each_unchecked_manual<
        'w,
        's,
        QF: Fetch<'w, 's, State = Q::State>,
        FN: Fn(QF::Item) + Send + Sync + Clone,
    >(
        &'s self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: FN,
        last_change_tick: u32,
        change_tick: u32,
    ) {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        task_pool.scope(|scope| {
            if QF::IS_DENSE && F::Fetch::IS_DENSE {
                let tables = &world.storages().tables;
                for table_id in self.matched_table_ids.iter() {
                    let table = &tables[*table_id];
                    let mut offset = 0;
                    while offset < table.len() {
                        let func = func.clone();
                        scope.spawn(async move {
                            let mut fetch =
                                QF::init(world, &self.fetch_state, last_change_tick, change_tick);
                            let mut filter = <F::Fetch as Fetch>::init(
                                world,
                                &self.filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let table = &tables[*table_id];
                            fetch.set_table(&self.fetch_state, table);
                            filter.set_table(&self.filter_state, table);
                            let len = batch_size.min(table.len() - offset);
                            for table_index in offset..offset + len {
                                if !filter.table_filter_fetch(table_index) {
                                    continue;
                                }
                                let item = fetch.table_fetch(table_index);
                                func(item);
                            }
                        });
                        offset += batch_size;
                    }
                }
            } else {
                let archetypes = &world.archetypes;
                for archetype_id in self.matched_archetype_ids.iter() {
                    let mut offset = 0;
                    let archetype = &archetypes[*archetype_id];
                    while offset < archetype.len() {
                        let func = func.clone();
                        scope.spawn(async move {
                            let mut fetch =
                                QF::init(world, &self.fetch_state, last_change_tick, change_tick);
                            let mut filter = <F::Fetch as Fetch>::init(
                                world,
                                &self.filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let archetype = &world.archetypes[*archetype_id];
                            fetch.set_archetype(&self.fetch_state, archetype, tables);
                            filter.set_archetype(&self.filter_state, archetype, tables);

                            let len = batch_size.min(archetype.len() - offset);
                            for archetype_index in offset..offset + len {
                                if !filter.archetype_filter_fetch(archetype_index) {
                                    continue;
                                }
                                func(fetch.archetype_fetch(archetype_index));
                            }
                        });
                        offset += batch_size;
                    }
                }
            }
        });
    }
}

/// An error that occurs when retrieving a specific [`Entity`]'s query result.
#[derive(Error, Debug)]
pub enum QueryEntityError {
    #[error("The given entity does not have the requested component.")]
    QueryDoesNotMatch,
    #[error("The requested entity does not exist.")]
    NoSuchEntity,
}
