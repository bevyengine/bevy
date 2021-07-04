use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::ComponentId,
    entity::Entity,
    query::{
        Access, Fetch, FetchState, FilterFetch, FilteredAccess, QueryCombinationIter, QueryIter,
        ReadOnlyFetch, WorldQuery,
    },
    storage::TableId,
    world::{World, WorldId},
};
use bevy_tasks::TaskPool;
use fixedbitset::FixedBitSet;
use thiserror::Error;

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
        state.validate_world_and_update_archetypes(world);
        state
    }

    #[inline]
    pub fn is_empty(&self, world: &World, last_change_tick: u32, change_tick: u32) -> bool {
        // SAFE: the iterator is instantly consumed via `none_remaining` and the implementation of
        // `QueryIter::none_remaining` never creates any references to the `<Q::Fetch as Fetch<'w>>::Item`.
        unsafe {
            self.iter_unchecked_manual(world, last_change_tick, change_tick)
                .none_remaining()
        }
    }

    pub fn validate_world_and_update_archetypes(&mut self, world: &World) {
        if world.id() != self.world_id {
            panic!("Attempted to use {} with a mismatched World. QueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>());
        }
        let archetypes = world.archetypes();
        let new_generation = archetypes.generation();
        let old_generation = std::mem::replace(&mut self.archetype_generation, new_generation);
        let archetype_index_range = old_generation.value()..new_generation.value();

        for archetype_index in archetype_index_range {
            self.new_archetype(&archetypes[ArchetypeId::new(archetype_index)]);
        }
    }

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

    #[inline]
    pub fn get<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFETY: query is read only
        unsafe { self.get_unchecked(world, entity) }
    }

    #[inline]
    pub fn get_mut<'w>(
        &mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
        // SAFETY: query has unique world access
        unsafe { self.get_unchecked(world, entity) }
    }

    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn get_unchecked<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
        self.validate_world_and_update_archetypes(world);
        self.get_unchecked_manual(
            world,
            entity,
            world.last_change_tick(),
            world.read_change_tick(),
        )
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub unsafe fn get_unchecked_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Result<<Q::Fetch as Fetch<'w>>::Item, QueryEntityError> {
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
        let mut fetch =
            <Q::Fetch as Fetch>::init(world, &self.fetch_state, last_change_tick, change_tick);
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

    #[inline]
    pub fn iter<'w, 's>(&'s mut self, world: &'w World) -> QueryIter<'w, 's, Q, F>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFETY: query is read only
        unsafe { self.iter_unchecked(world) }
    }

    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, Q, F> {
        // SAFETY: query has unique world access
        unsafe { self.iter_unchecked(world) }
    }

    #[inline]
    pub fn iter_combinations<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w World,
    ) -> QueryCombinationIter<'w, 's, Q, F, K>
    where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFE: query is read only
        unsafe { self.iter_combinations_unchecked(world) }
    }

    #[inline]
    pub fn iter_combinations_mut<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w mut World,
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
        // SAFE: query has unique world access
        unsafe { self.iter_combinations_unchecked(world) }
    }

    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_unchecked<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryIter<'w, 's, Q, F> {
        self.validate_world_and_update_archetypes(world);
        self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
    }

    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_combinations_unchecked<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w World,
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
        self.validate_world_and_update_archetypes(world);
        self.iter_combinations_unchecked_manual(
            world,
            world.last_change_tick(),
            world.read_change_tick(),
        )
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsound.
    #[inline]
    pub(crate) unsafe fn iter_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryIter<'w, 's, Q, F> {
        QueryIter::new(world, self, last_change_tick, change_tick)
    }

    /// # Safety
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsound.
    #[inline]
    pub(crate) unsafe fn iter_combinations_unchecked_manual<'w, 's, const K: usize>(
        &'s self,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
        QueryCombinationIter::new(world, self, last_change_tick, change_tick)
    }

    #[inline]
    pub fn for_each<'w>(
        &mut self,
        world: &'w World,
        func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
    ) where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFETY: query is read only
        unsafe {
            self.for_each_unchecked(world, func);
        }
    }

    #[inline]
    pub fn for_each_mut<'w>(
        &mut self,
        world: &'w mut World,
        func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
    ) {
        // SAFETY: query has unique world access
        unsafe {
            self.for_each_unchecked(world, func);
        }
    }

    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn for_each_unchecked<'w>(
        &mut self,
        world: &'w World,
        func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
    ) {
        self.validate_world_and_update_archetypes(world);
        self.for_each_unchecked_manual(
            world,
            func,
            world.last_change_tick(),
            world.read_change_tick(),
        );
    }

    #[inline]
    pub fn par_for_each<'w>(
        &mut self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) where
        Q::Fetch: ReadOnlyFetch,
    {
        // SAFETY: query is read only
        unsafe {
            self.par_for_each_unchecked(world, task_pool, batch_size, func);
        }
    }

    #[inline]
    pub fn par_for_each_mut<'w>(
        &mut self,
        world: &'w mut World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) {
        // SAFETY: query has unique world access
        unsafe {
            self.par_for_each_unchecked(world, task_pool, batch_size, func);
        }
    }

    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn par_for_each_unchecked<'w>(
        &mut self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
    ) {
        self.validate_world_and_update_archetypes(world);
        self.par_for_each_unchecked_manual(
            world,
            task_pool,
            batch_size,
            func,
            world.last_change_tick(),
            world.read_change_tick(),
        );
    }

    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsound.
    pub(crate) unsafe fn for_each_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        mut func: impl FnMut(<Q::Fetch as Fetch<'w>>::Item),
        last_change_tick: u32,
        change_tick: u32,
    ) {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        let mut fetch =
            <Q::Fetch as Fetch>::init(world, &self.fetch_state, last_change_tick, change_tick);
        let mut filter =
            <F::Fetch as Fetch>::init(world, &self.filter_state, last_change_tick, change_tick);
        if fetch.is_dense() && filter.is_dense() {
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

    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched WorldId is unsound.
    pub unsafe fn par_for_each_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        task_pool: &TaskPool,
        batch_size: usize,
        func: impl Fn(<Q::Fetch as Fetch<'w>>::Item) + Send + Sync + Clone,
        last_change_tick: u32,
        change_tick: u32,
    ) {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        task_pool.scope(|scope| {
            let fetch =
                <Q::Fetch as Fetch>::init(world, &self.fetch_state, last_change_tick, change_tick);
            let filter =
                <F::Fetch as Fetch>::init(world, &self.filter_state, last_change_tick, change_tick);

            if fetch.is_dense() && filter.is_dense() {
                let tables = &world.storages().tables;
                for table_id in self.matched_table_ids.iter() {
                    let table = &tables[*table_id];
                    let mut offset = 0;
                    while offset < table.len() {
                        let func = func.clone();
                        scope.spawn(async move {
                            let mut fetch = <Q::Fetch as Fetch>::init(
                                world,
                                &self.fetch_state,
                                last_change_tick,
                                change_tick,
                            );
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
                            let mut fetch = <Q::Fetch as Fetch>::init(
                                world,
                                &self.fetch_state,
                                last_change_tick,
                                change_tick,
                            );
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
