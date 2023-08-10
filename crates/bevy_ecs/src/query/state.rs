use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::{ComponentId, Tick},
    entity::Entity,
    prelude::FromWorld,
    query::{
        Access, BatchingStrategy, DebugCheckedUnwrap, FilteredAccess, QueryCombinationIter,
        QueryIter, QueryParIter, WorldQuery,
    },
    storage::{TableId, TableRow},
    world::{unsafe_world_cell::UnsafeWorldCell, World, WorldId},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::Instrument;
use fixedbitset::FixedBitSet;
use std::{borrow::Borrow, fmt, mem::MaybeUninit};

use super::{NopWorldQuery, QueryManyIter, ROQueryItem, ReadOnlyWorldQuery};

/// Provides scoped access to a [`World`] state according to a given [`WorldQuery`] and query filter.
#[repr(C)]
// SAFETY NOTE:
// Do not add any new fields that use the `Q` or `F` generic parameters as this may
// make `QueryState::as_transmuted_state` unsound if not done with care.
pub struct QueryState<Q: WorldQuery, F: ReadOnlyWorldQuery = ()> {
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

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> std::fmt::Debug for QueryState<Q, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "QueryState<Q, F> matched_table_ids: {} matched_archetype_ids: {}",
            self.matched_table_ids.len(),
            self.matched_archetype_ids.len()
        )
    }
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> FromWorld for QueryState<Q, F> {
    fn from_world(world: &mut World) -> Self {
        world.query_filtered()
    }
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> QueryState<Q, F> {
    /// Converts this `QueryState` reference to a `QueryState` that does not access anything mutably.
    pub fn as_readonly(&self) -> &QueryState<Q::ReadOnly, F::ReadOnly> {
        // SAFETY: invariant on `WorldQuery` trait upholds that `Q::ReadOnly` and `F::ReadOnly`
        // have a subset of the access, and match the exact same archetypes/tables as `Q`/`F` respectively.
        unsafe { self.as_transmuted_state::<Q::ReadOnly, F::ReadOnly>() }
    }

    /// Converts this `QueryState` reference to a `QueryState` that does not return any data
    /// which can be faster.
    ///
    /// This doesn't use `NopWorldQuery` as it loses filter functionality, for example
    /// `NopWorldQuery<Changed<T>>` is functionally equivalent to `With<T>`.
    pub fn as_nop(&self) -> &QueryState<NopWorldQuery<Q>, F> {
        // SAFETY: `NopWorldQuery` doesn't have any accesses and defers to
        // `Q` for table/archetype matching
        unsafe { self.as_transmuted_state::<NopWorldQuery<Q>, F>() }
    }

    /// Converts this `QueryState` reference to any other `QueryState` with
    /// the same `WorldQuery::State` associated types.
    ///
    /// Consider using `as_readonly` or `as_nop` instead which are safe functions.
    ///
    /// # SAFETY
    ///
    /// `NewQ` must have a subset of the access that `Q` does and match the exact same archetypes/tables
    /// `NewF` must have a subset of the access that `F` does and match the exact same archetypes/tables
    pub(crate) unsafe fn as_transmuted_state<
        NewQ: WorldQuery<State = Q::State>,
        NewF: ReadOnlyWorldQuery<State = F::State>,
    >(
        &self,
    ) -> &QueryState<NewQ, NewF> {
        &*(self as *const QueryState<Q, F> as *const QueryState<NewQ, NewF>)
    }
}

impl<Q: WorldQuery, F: ReadOnlyWorldQuery> QueryState<Q, F> {
    /// Creates a new [`QueryState`] from a given [`World`] and inherits the result of `world.id()`.
    pub fn new(world: &mut World) -> Self {
        let fetch_state = Q::init_state(world);
        let filter_state = F::init_state(world);

        let mut component_access = FilteredAccess::default();
        Q::update_component_access(&fetch_state, &mut component_access);

        // Use a temporary empty FilteredAccess for filters. This prevents them from conflicting with the
        // main Query's `fetch_state` access. Filters are allowed to conflict with the main query fetch
        // because they are evaluated *before* a specific reference is constructed.
        let mut filter_component_access = FilteredAccess::default();
        F::update_component_access(&filter_state, &mut filter_component_access);

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
    pub fn is_empty(&self, world: &World, last_run: Tick, this_run: Tick) -> bool {
        // SAFETY: NopFetch does not access any members while &self ensures no one has exclusive access
        unsafe {
            self.as_nop()
                .iter_unchecked_manual(world.as_unsafe_world_cell_readonly(), last_run, this_run)
                .next()
                .is_none()
        }
    }

    /// Updates the state's internal view of the [`World`]'s archetypes. If this is not called before querying data,
    /// the results may not accurately reflect what is in the `world`.
    ///
    /// This is only required if a `manual` method (such as [`Self::get_manual`]) is being called, and it only needs to
    /// be called if the `world` has been structurally mutated (i.e. added/removed a component or resource). Users using
    /// non-`manual` methods such as [`QueryState::get`] do not need to call this as it will be automatically called for them.
    ///
    /// If you have an [`UnsafeWorldCell`] instead of `&World`, consider using [`QueryState::update_archetypes_unsafe_world_cell`].
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
    /// non-`manual` methods such as [`QueryState::get`] do not need to call this as it will be automatically called for them.
    ///
    /// # Note
    ///
    /// This method only accesses world metadata.
    ///
    /// # Panics
    ///
    /// If `world` does not match the one used to call `QueryState::new` for this instance.
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
            "Attempted to use {} with a mismatched World. QueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>(),
        );
    }

    /// Update the current [`QueryState`] with information from the provided [`Archetype`]
    /// (if applicable, i.e. if the archetype has any intersecting [`ComponentId`] with the current [`QueryState`]).
    pub fn new_archetype(&mut self, archetype: &Archetype) {
        if Q::matches_component_set(&self.fetch_state, &|id| archetype.contains(id))
            && F::matches_component_set(&self.filter_state, &|id| archetype.contains(id))
        {
            Q::update_archetype_component_access(
                &self.fetch_state,
                archetype,
                &mut self.archetype_component_access,
            );
            F::update_archetype_component_access(
                &self.filter_state,
                archetype,
                &mut self.archetype_component_access,
            );
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
    pub fn get<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, Q>, QueryEntityError> {
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

    /// Returns the read-only query results for the given array of [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// Note that the unlike [`QueryState::get_many_mut`], the entities passed in do not need to be unique.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::query::QueryEntityError;
    ///
    /// #[derive(Component, PartialEq, Debug)]
    /// struct A(usize);
    ///
    /// let mut world = World::new();
    /// let entity_vec: Vec<Entity> = (0..3).map(|i|world.spawn(A(i)).id()).collect();
    /// let entities: [Entity; 3] = entity_vec.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    ///
    /// let mut query_state = world.query::<&A>();
    ///
    /// let component_values = query_state.get_many(&world, entities).unwrap();
    ///
    /// assert_eq!(component_values, [&A(0), &A(1), &A(2)]);
    ///
    /// let wrong_entity = Entity::from_raw(365);
    ///
    /// assert_eq!(query_state.get_many(&world, [wrong_entity]), Err(QueryEntityError::NoSuchEntity(wrong_entity)));
    /// ```
    #[inline]
    pub fn get_many<'w, const N: usize>(
        &mut self,
        world: &'w World,
        entities: [Entity; N],
    ) -> Result<[ROQueryItem<'w, Q>; N], QueryEntityError> {
        self.update_archetypes(world);

        // SAFETY: update_archetypes validates the `World` matches
        unsafe {
            self.get_many_read_only_manual(
                world,
                entities,
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

    /// Returns the query results for the given array of [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::query::QueryEntityError;
    ///
    /// #[derive(Component, PartialEq, Debug)]
    /// struct A(usize);
    ///
    /// let mut world = World::new();
    ///
    /// let entities: Vec<Entity> = (0..3).map(|i|world.spawn(A(i)).id()).collect();
    /// let entities: [Entity; 3] = entities.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    ///
    /// let mut query_state = world.query::<&mut A>();
    ///
    /// let mut mutable_component_values = query_state.get_many_mut(&mut world, entities).unwrap();
    ///
    /// for mut a in &mut mutable_component_values {
    ///     a.0 += 5;
    /// }
    ///
    /// let component_values = query_state.get_many(&world, entities).unwrap();
    ///
    /// assert_eq!(component_values, [&A(5), &A(6), &A(7)]);
    ///
    /// let wrong_entity = Entity::from_raw(57);
    /// let invalid_entity = world.spawn_empty().id();
    ///
    /// assert_eq!(query_state.get_many_mut(&mut world, [wrong_entity]).unwrap_err(), QueryEntityError::NoSuchEntity(wrong_entity));
    /// assert_eq!(query_state.get_many_mut(&mut world, [invalid_entity]).unwrap_err(), QueryEntityError::QueryDoesNotMatch(invalid_entity));
    /// assert_eq!(query_state.get_many_mut(&mut world, [entities[0], entities[0]]).unwrap_err(), QueryEntityError::AliasedMutability(entities[0]));
    /// ```
    #[inline]
    pub fn get_many_mut<'w, const N: usize>(
        &mut self,
        world: &'w mut World,
        entities: [Entity; N],
    ) -> Result<[Q::Item<'w>; N], QueryEntityError> {
        self.update_archetypes(world);

        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: method requires exclusive world access
        // and world has been validated via update_archetypes
        unsafe {
            self.get_many_unchecked_manual(
                world.as_unsafe_world_cell(),
                entities,
                last_change_tick,
                change_tick,
            )
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This method is slightly more efficient than [`QueryState::get`] in some situations, since
    /// it does not update this instance's internal cache. This method will return an error if `entity`
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], `QueryState::get`, or any method with mutable
    /// access to `self`.
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for mutable queries.
    #[inline]
    pub fn get_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, Q>, QueryEntityError> {
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
    /// This must be called on the same `World` that the `Query` was generated from:
    /// use `QueryState::validate_world` to verify this.
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
        let archetype = world
            .archetypes()
            .get(location.archetype_id)
            .debug_checked_unwrap();
        let mut fetch = Q::init_fetch(world, &self.fetch_state, last_run, this_run);
        let mut filter = F::init_fetch(world, &self.filter_state, last_run, this_run);

        let table = world
            .storages()
            .tables
            .get(location.table_id)
            .debug_checked_unwrap();
        Q::set_archetype(&mut fetch, &self.fetch_state, archetype, table);
        F::set_archetype(&mut filter, &self.filter_state, archetype, table);

        if F::filter_fetch(&mut filter, entity, location.table_row) {
            Ok(Q::fetch(&mut fetch, entity, location.table_row))
        } else {
            Err(QueryEntityError::QueryDoesNotMatch(entity))
        }
    }

    /// Gets the read-only query results for the given [`World`] and array of [`Entity`], where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// This must be called on the same `World` that the `Query` was generated from:
    /// use `QueryState::validate_world` to verify this.
    pub(crate) unsafe fn get_many_read_only_manual<'w, const N: usize>(
        &self,
        world: &'w World,
        entities: [Entity; N],
        last_run: Tick,
        this_run: Tick,
    ) -> Result<[ROQueryItem<'w, Q>; N], QueryEntityError> {
        let mut values = [(); N].map(|_| MaybeUninit::uninit());

        for (value, entity) in std::iter::zip(&mut values, entities) {
            // SAFETY: fetch is read-only
            // and world must be validated
            let item = self.as_readonly().get_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                entity,
                last_run,
                this_run,
            )?;
            *value = MaybeUninit::new(item);
        }

        // SAFETY: Each value has been fully initialized.
        Ok(values.map(|x| x.assume_init()))
    }

    /// Gets the query results for the given [`World`] and array of [`Entity`], where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// This does not check for unique access to subsets of the entity-component data.
    /// To be safe, make sure mutable queries have unique access to the components they query.
    ///
    /// This must be called on the same `World` that the `Query` was generated from:
    /// use `QueryState::validate_world` to verify this.
    pub(crate) unsafe fn get_many_unchecked_manual<'w, const N: usize>(
        &self,
        world: UnsafeWorldCell<'w>,
        entities: [Entity; N],
        last_run: Tick,
        this_run: Tick,
    ) -> Result<[Q::Item<'w>; N], QueryEntityError> {
        // Verify that all entities are unique
        for i in 0..N {
            for j in 0..i {
                if entities[i] == entities[j] {
                    return Err(QueryEntityError::AliasedMutability(entities[i]));
                }
            }
        }

        let mut values = [(); N].map(|_| MaybeUninit::uninit());

        for (value, entity) in std::iter::zip(&mut values, entities) {
            let item = self.get_unchecked_manual(world, entity, last_run, this_run)?;
            *value = MaybeUninit::new(item);
        }

        // SAFETY: Each value has been fully initialized.
        Ok(values.map(|x| x.assume_init()))
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`Self::iter_mut`] for write-queries.
    #[inline]
    pub fn iter<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly> {
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
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, Q, F> {
        self.update_archetypes(world);
        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: query has unique world access
        unsafe {
            self.iter_unchecked_manual(world.as_unsafe_world_cell(), last_change_tick, change_tick)
        }
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`] without updating the query's archetypes.
    /// Archetypes must be manually updated before by using [`Self::update_archetypes`].
    ///
    /// This can only be called for read-only queries.
    #[inline]
    pub fn iter_manual<'w, 's>(
        &'s self,
        world: &'w World,
    ) -> QueryIter<'w, 's, Q::ReadOnly, F::ReadOnly> {
        self.validate_world(world.id());
        // SAFETY: query is read only and world is validated
        unsafe {
            self.as_readonly().iter_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    /// This can only be called for read-only queries.
    ///
    /// A combination is an arrangement of a collection of items where order does not matter.
    ///
    /// `K` is the number of items that make up each subset, and the number of items returned by the iterator.
    /// `N` is the number of total entities output by query.
    ///
    /// For example, given the list [1, 2, 3, 4], where `K` is 2, the combinations without repeats are
    /// [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4].
    /// And in this case, `N` would be defined as 4 since the size of the input list is 4.
    ///
    ///  For combinations of size `K` of query taking `N` inputs, you will get:
    /// - if `K == N`: one combination of all query results
    /// - if `K < N`: all possible `K`-sized combinations of query results, without repetition
    /// - if `K > N`: empty set (no `K`-sized combinations exist)
    ///
    /// The `iter_combinations` method does not guarantee order of iteration.
    ///
    /// This can only be called for read-only queries, see [`Self::iter_combinations_mut`] for
    /// write-queries.
    #[inline]
    pub fn iter_combinations<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w World,
    ) -> QueryCombinationIter<'w, 's, Q::ReadOnly, F::ReadOnly, K> {
        self.update_archetypes(world);
        // SAFETY: query is read only
        unsafe {
            self.as_readonly().iter_combinations_unchecked_manual(
                world.as_unsafe_world_cell_readonly(),
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results without repetition.
    ///
    /// A combination is an arrangement of a collection of items where order does not matter.
    ///
    /// `K` is the number of items that make up each subset, and the number of items returned by the iterator.
    /// `N` is the number of total entities output by query.
    ///
    /// For example, given the list [1, 2, 3, 4], where `K` is 2, the combinations without repeats are
    /// [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4].
    /// And in this case, `N` would be defined as 4 since the size of the input list is 4.
    ///
    ///  For combinations of size `K` of query taking `N` inputs, you will get:
    /// - if `K == N`: one combination of all query results
    /// - if `K < N`: all possible `K`-sized combinations of query results, without repetition
    /// - if `K > N`: empty set (no `K`-sized combinations exist)
    ///
    /// The `iter_combinations_mut` method does not guarantee order of iteration.
    #[inline]
    pub fn iter_combinations_mut<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w mut World,
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
        self.update_archetypes(world);
        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: query has unique world access
        unsafe {
            self.iter_combinations_unchecked_manual(
                world.as_unsafe_world_cell(),
                last_change_tick,
                change_tick,
            )
        }
    }

    /// Returns an [`Iterator`] over the read-only query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    ///
    /// # See also
    ///
    /// - [`iter_many_mut`](Self::iter_many_mut) to get mutable query items.
    #[inline]
    pub fn iter_many<'w, 's, EntityList: IntoIterator>(
        &'s mut self,
        world: &'w World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, Q::ReadOnly, F::ReadOnly, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        self.update_archetypes(world);
        // SAFETY: query is read only
        unsafe {
            self.as_readonly().iter_many_unchecked_manual(
                entities,
                world.as_unsafe_world_cell_readonly(),
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns an [`Iterator`] over the read-only query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    ///
    /// If `world` archetypes changed since [`Self::update_archetypes`] was last called,
    /// this will skip entities contained in new archetypes.
    ///
    /// This can only be called for read-only queries.
    ///
    /// # See also
    ///
    /// - [`iter_many`](Self::iter_many) to update archetypes.
    /// - [`iter_manual`](Self::iter_manual) to iterate over all query items.
    #[inline]
    pub fn iter_many_manual<'w, 's, EntityList: IntoIterator>(
        &'s self,
        world: &'w World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, Q::ReadOnly, F::ReadOnly, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        self.validate_world(world.id());
        // SAFETY: query is read only, world id is validated
        unsafe {
            self.as_readonly().iter_many_unchecked_manual(
                entities,
                world.as_unsafe_world_cell_readonly(),
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns an iterator over the query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    #[inline]
    pub fn iter_many_mut<'w, 's, EntityList: IntoIterator>(
        &'s mut self,
        world: &'w mut World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, Q, F, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        self.update_archetypes(world);
        let change_tick = world.change_tick();
        let last_change_tick = world.last_change_tick();
        // SAFETY: Query has unique world access.
        unsafe {
            self.iter_many_unchecked_manual(
                entities,
                world.as_unsafe_world_cell(),
                last_change_tick,
                change_tick,
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
        world: UnsafeWorldCell<'w>,
    ) -> QueryIter<'w, 's, Q, F> {
        self.update_archetypes_unsafe_world_cell(world);
        self.iter_unchecked_manual(world, world.last_change_tick(), world.change_tick())
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
        world: UnsafeWorldCell<'w>,
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
        self.update_archetypes_unsafe_world_cell(world);
        self.iter_combinations_unchecked_manual(
            world,
            world.last_change_tick(),
            world.change_tick(),
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
    pub(crate) unsafe fn iter_unchecked_manual<'w, 's>(
        &'s self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> QueryIter<'w, 's, Q, F> {
        QueryIter::new(world, self, last_run, this_run)
    }

    /// Returns an [`Iterator`] for the given [`World`] and list of [`Entity`]'s, where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not check for entity uniqueness
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    #[inline]
    pub(crate) unsafe fn iter_many_unchecked_manual<'w, 's, EntityList: IntoIterator>(
        &'s self,
        entities: EntityList,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> QueryManyIter<'w, 's, Q, F, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        QueryManyIter::new(world, self, entities, last_run, this_run)
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
    pub(crate) unsafe fn iter_combinations_unchecked_manual<'w, 's, const K: usize>(
        &'s self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
        QueryCombinationIter::new(world, self, last_run, this_run)
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// This can only be called for read-only queries, see [`Self::for_each_mut`] for write-queries.
    #[inline]
    pub fn for_each<'w, FN: FnMut(ROQueryItem<'w, Q>)>(&mut self, world: &'w World, func: FN) {
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

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`par_iter_mut`] for write-queries.
    ///
    /// [`par_iter_mut`]: Self::par_iter_mut
    #[inline]
    pub fn par_iter<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryParIter<'w, 's, Q::ReadOnly, F::ReadOnly> {
        self.update_archetypes(world);
        QueryParIter {
            world: world.as_unsafe_world_cell_readonly(),
            state: self.as_readonly(),
            last_run: world.last_change_tick(),
            this_run: world.read_change_tick(),
            batching_strategy: BatchingStrategy::new(),
        }
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for mutable queries, see [`par_iter`] for read-only-queries.
    ///
    /// [`par_iter`]: Self::par_iter
    #[inline]
    pub fn par_iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryParIter<'w, 's, Q, F> {
        self.update_archetypes(world);
        let this_run = world.change_tick();
        let last_run = world.last_change_tick();
        QueryParIter {
            world: world.as_unsafe_world_cell(),
            state: self,
            last_run,
            this_run,
            batching_strategy: BatchingStrategy::new(),
        }
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
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        let mut fetch = Q::init_fetch(world, &self.fetch_state, last_run, this_run);
        let mut filter = F::init_fetch(world, &self.filter_state, last_run, this_run);

        let tables = &world.storages().tables;
        if Q::IS_DENSE && F::IS_DENSE {
            for table_id in &self.matched_table_ids {
                let table = tables.get(*table_id).debug_checked_unwrap();
                Q::set_table(&mut fetch, &self.fetch_state, table);
                F::set_table(&mut filter, &self.filter_state, table);

                let entities = table.entities();
                for row in 0..table.entity_count() {
                    let entity = entities.get_unchecked(row);
                    let row = TableRow::new(row);
                    if !F::filter_fetch(&mut filter, *entity, row) {
                        continue;
                    }
                    func(Q::fetch(&mut fetch, *entity, row));
                }
            }
        } else {
            let archetypes = world.archetypes();
            for archetype_id in &self.matched_archetype_ids {
                let archetype = archetypes.get(*archetype_id).debug_checked_unwrap();
                let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                Q::set_archetype(&mut fetch, &self.fetch_state, archetype, table);
                F::set_archetype(&mut filter, &self.filter_state, archetype, table);

                let entities = archetype.entities();
                for idx in 0..archetype.len() {
                    let archetype_entity = entities.get_unchecked(idx);
                    if !F::filter_fetch(
                        &mut filter,
                        archetype_entity.entity(),
                        archetype_entity.table_row(),
                    ) {
                        continue;
                    }
                    func(Q::fetch(
                        &mut fetch,
                        archetype_entity.entity(),
                        archetype_entity.table_row(),
                    ));
                }
            }
        }
    }

    /// Runs `func` on each query result in parallel for the given [`World`], where the last change and
    /// the current change tick are given. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    ///
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[cfg(all(not(target = "wasm32"), feature = "multi-threaded"))]
    pub(crate) unsafe fn par_for_each_unchecked_manual<
        'w,
        FN: Fn(Q::Item<'w>) + Send + Sync + Clone,
    >(
        &self,
        world: UnsafeWorldCell<'w>,
        batch_size: usize,
        func: FN,
        last_run: Tick,
        this_run: Tick,
    ) {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        bevy_tasks::ComputeTaskPool::get().scope(|scope| {
            if Q::IS_DENSE && F::IS_DENSE {
                // SAFETY: We only access table data that has been registered in `self.archetype_component_access`.
                let tables = &world.storages().tables;
                for table_id in &self.matched_table_ids {
                    let table = &tables[*table_id];
                    if table.is_empty() {
                        continue;
                    }

                    let mut offset = 0;
                    while offset < table.entity_count() {
                        let func = func.clone();
                        let len = batch_size.min(table.entity_count() - offset);
                        let task = async move {
                            let mut fetch =
                                Q::init_fetch(world, &self.fetch_state, last_run, this_run);
                            let mut filter =
                                F::init_fetch(world, &self.filter_state, last_run, this_run);
                            let tables = &world.storages().tables;
                            let table = tables.get(*table_id).debug_checked_unwrap();
                            let entities = table.entities();
                            Q::set_table(&mut fetch, &self.fetch_state, table);
                            F::set_table(&mut filter, &self.filter_state, table);
                            for row in offset..offset + len {
                                let entity = entities.get_unchecked(row);
                                let row = TableRow::new(row);
                                if !F::filter_fetch(&mut filter, *entity, row) {
                                    continue;
                                }
                                func(Q::fetch(&mut fetch, *entity, row));
                            }
                        };
                        #[cfg(feature = "trace")]
                        let span = bevy_utils::tracing::info_span!(
                            "par_for_each",
                            query = std::any::type_name::<Q>(),
                            filter = std::any::type_name::<F>(),
                            count = len,
                        );
                        #[cfg(feature = "trace")]
                        let task = task.instrument(span);
                        scope.spawn(task);
                        offset += batch_size;
                    }
                }
            } else {
                let archetypes = world.archetypes();
                for archetype_id in &self.matched_archetype_ids {
                    let mut offset = 0;
                    let archetype = &archetypes[*archetype_id];
                    if archetype.is_empty() {
                        continue;
                    }

                    while offset < archetype.len() {
                        let func = func.clone();
                        let len = batch_size.min(archetype.len() - offset);
                        let task = async move {
                            let mut fetch =
                                Q::init_fetch(world, &self.fetch_state, last_run, this_run);
                            let mut filter =
                                F::init_fetch(world, &self.filter_state, last_run, this_run);
                            let tables = &world.storages().tables;
                            let archetype =
                                world.archetypes().get(*archetype_id).debug_checked_unwrap();
                            let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                            Q::set_archetype(&mut fetch, &self.fetch_state, archetype, table);
                            F::set_archetype(&mut filter, &self.filter_state, archetype, table);

                            let entities = archetype.entities();
                            for archetype_row in offset..offset + len {
                                let archetype_entity = entities.get_unchecked(archetype_row);
                                if !F::filter_fetch(
                                    &mut filter,
                                    archetype_entity.entity(),
                                    archetype_entity.table_row(),
                                ) {
                                    continue;
                                }
                                func(Q::fetch(
                                    &mut fetch,
                                    archetype_entity.entity(),
                                    archetype_entity.table_row(),
                                ));
                            }
                        };

                        #[cfg(feature = "trace")]
                        let span = bevy_utils::tracing::info_span!(
                            "par_for_each",
                            query = std::any::type_name::<Q>(),
                            filter = std::any::type_name::<F>(),
                            count = len,
                        );
                        #[cfg(feature = "trace")]
                        let task = task.instrument(span);

                        scope.spawn(task);
                        offset += batch_size;
                    }
                }
            }
        });
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
    pub fn single<'w>(&mut self, world: &'w World) -> ROQueryItem<'w, Q> {
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
    ) -> Result<ROQueryItem<'w, Q>, QuerySingleError> {
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
}

/// An error that occurs when retrieving a specific [`Entity`]'s query result from [`Query`](crate::system::Query) or [`QueryState`].
// TODO: return the type_name as part of this error
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QueryEntityError {
    /// The given [`Entity`]'s components do not match the query.
    ///
    /// Either it does not have a requested component, or it has a component which the query filters out.
    QueryDoesNotMatch(Entity),
    /// The given [`Entity`] does not exist.
    NoSuchEntity(Entity),
    /// The [`Entity`] was requested mutably more than once.
    ///
    /// See [`QueryState::get_many_mut`] for an example.
    AliasedMutability(Entity),
}

impl std::error::Error for QueryEntityError {}

impl fmt::Display for QueryEntityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueryEntityError::QueryDoesNotMatch(_) => {
                write!(f, "The given entity's components do not match the query.")
            }
            QueryEntityError::NoSuchEntity(_) => write!(f, "The requested entity does not exist."),
            QueryEntityError::AliasedMutability(_) => {
                write!(f, "The entity was requested mutably more than once.")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, query::QueryEntityError};

    #[test]
    fn get_many_unchecked_manual_uniqueness() {
        let mut world = World::new();

        let entities: Vec<Entity> = (0..10).map(|_| world.spawn_empty().id()).collect();

        let query_state = world.query::<Entity>();

        // These don't matter for the test
        let last_change_tick = world.last_change_tick();
        let change_tick = world.change_tick();

        // It's best to test get_many_unchecked_manual directly,
        // as it is shared and unsafe
        // We don't care about aliased mutability for the read-only equivalent

        // SAFETY: Query does not access world data.
        assert!(unsafe {
            query_state
                .get_many_unchecked_manual::<10>(
                    world.as_unsafe_world_cell_readonly(),
                    entities.clone().try_into().unwrap(),
                    last_change_tick,
                    change_tick,
                )
                .is_ok()
        });

        assert_eq!(
            // SAFETY: Query does not access world data.
            unsafe {
                query_state
                    .get_many_unchecked_manual(
                        world.as_unsafe_world_cell_readonly(),
                        [entities[0], entities[0]],
                        last_change_tick,
                        change_tick,
                    )
                    .unwrap_err()
            },
            QueryEntityError::AliasedMutability(entities[0])
        );

        assert_eq!(
            // SAFETY: Query does not access world data.
            unsafe {
                query_state
                    .get_many_unchecked_manual(
                        world.as_unsafe_world_cell_readonly(),
                        [entities[0], entities[1], entities[0]],
                        last_change_tick,
                        change_tick,
                    )
                    .unwrap_err()
            },
            QueryEntityError::AliasedMutability(entities[0])
        );

        assert_eq!(
            // SAFETY: Query does not access world data.
            unsafe {
                query_state
                    .get_many_unchecked_manual(
                        world.as_unsafe_world_cell_readonly(),
                        [entities[9], entities[9]],
                        last_change_tick,
                        change_tick,
                    )
                    .unwrap_err()
            },
            QueryEntityError::AliasedMutability(entities[9])
        );
    }

    #[test]
    #[should_panic]
    fn right_world_get() {
        let mut world_1 = World::new();
        let world_2 = World::new();

        let mut query_state = world_1.query::<Entity>();
        let _panics = query_state.get(&world_2, Entity::from_raw(0));
    }

    #[test]
    #[should_panic]
    fn right_world_get_many() {
        let mut world_1 = World::new();
        let world_2 = World::new();

        let mut query_state = world_1.query::<Entity>();
        let _panics = query_state.get_many(&world_2, []);
    }

    #[test]
    #[should_panic]
    fn right_world_get_many_mut() {
        let mut world_1 = World::new();
        let mut world_2 = World::new();

        let mut query_state = world_1.query::<Entity>();
        let _panics = query_state.get_many_mut(&mut world_2, []);
    }
}

/// An error that occurs when evaluating a [`Query`](crate::system::Query) or [`QueryState`] as a single expected result via
/// [`get_single`](crate::system::Query::get_single) or [`get_single_mut`](crate::system::Query::get_single_mut).
#[derive(Debug)]
pub enum QuerySingleError {
    /// No entity fits the query.
    NoEntities(&'static str),
    /// Multiple entities fit the query.
    MultipleEntities(&'static str),
}

impl std::error::Error for QuerySingleError {}

impl std::fmt::Display for QuerySingleError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QuerySingleError::NoEntities(query) => write!(f, "No entities fit the query {query}"),
            QuerySingleError::MultipleEntities(query) => {
                write!(f, "Multiple entities fit the query {query}!")
            }
        }
    }
}
