use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    component::ComponentId,
    entity::Entity,
    prelude::FromWorld,
    query::{
        Access, DebugCheckedUnwrap, FilteredAccess, QueryCombinationIter, QueryIter, WorldQuery,
    },
    storage::TableId,
    world::{World, WorldId},
};
use bevy_tasks::ComputeTaskPool;
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
    pub fn is_empty(&self, world: &World, last_change_tick: u32, change_tick: u32) -> bool {
        // SAFETY: NopFetch does not access any members while &self ensures no one has exclusive access
        unsafe {
            self.as_nop()
                .iter_unchecked_manual(world, last_change_tick, change_tick)
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
        assert!(
            world.id() == self.world_id,
            "Attempted to use {} with a mismatched World. QueryStates can only be used with the World they were created from.",
                std::any::type_name::<Self>(),
        );
    }

    /// Creates a new [`Archetype`].
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
                world,
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
        // SAFETY: query has unique world access
        unsafe {
            self.get_unchecked_manual(
                world,
                entity,
                world.last_change_tick(),
                world.read_change_tick(),
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

        // SAFETY: method requires exclusive world access
        // and world has been validated via update_archetypes
        unsafe {
            self.get_many_unchecked_manual(
                world,
                entities,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    #[inline]
    pub fn get_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, Q>, QueryEntityError> {
        self.validate_world(world);
        // SAFETY: query is read only and world is validated
        unsafe {
            self.as_readonly().get_unchecked_manual(
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
    pub unsafe fn get_unchecked<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<Q::Item<'w>, QueryEntityError> {
        self.update_archetypes(world);
        self.get_unchecked_manual(
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
    ///
    /// This must be called on the same `World` that the `Query` was generated from:
    /// use `QueryState::validate_world` to verify this.
    pub(crate) unsafe fn get_unchecked_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Result<Q::Item<'w>, QueryEntityError> {
        let location = world
            .entities
            .get(entity)
            .ok_or(QueryEntityError::NoSuchEntity(entity))?;
        if !self
            .matched_archetypes
            .contains(location.archetype_id.index())
        {
            return Err(QueryEntityError::QueryDoesNotMatch(entity));
        }
        let archetype = world
            .archetypes
            .get(location.archetype_id)
            .debug_checked_unwrap();
        let mut fetch = Q::init_fetch(world, &self.fetch_state, last_change_tick, change_tick);
        let mut filter = F::init_fetch(world, &self.filter_state, last_change_tick, change_tick);

        let table_row = archetype.entity_table_row(location.index);
        let table = world
            .storages()
            .tables
            .get(archetype.table_id())
            .debug_checked_unwrap();
        Q::set_archetype(&mut fetch, &self.fetch_state, archetype, table);
        F::set_archetype(&mut filter, &self.filter_state, archetype, table);

        if F::filter_fetch(&mut filter, entity, table_row) {
            Ok(Q::fetch(&mut fetch, entity, table_row))
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
        last_change_tick: u32,
        change_tick: u32,
    ) -> Result<[ROQueryItem<'w, Q>; N], QueryEntityError> {
        let mut values = [(); N].map(|_| MaybeUninit::uninit());

        for (value, entity) in std::iter::zip(&mut values, entities) {
            // SAFETY: fetch is read-only
            // and world must be validated
            let item = self.as_readonly().get_unchecked_manual(
                world,
                entity,
                last_change_tick,
                change_tick,
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
        world: &'w World,
        entities: [Entity; N],
        last_change_tick: u32,
        change_tick: u32,
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
            let item = self.get_unchecked_manual(world, entity, last_change_tick, change_tick)?;
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
        // SAFETY: query is read only
        unsafe {
            self.update_archetypes(world);
            self.as_readonly().iter_unchecked_manual(
                world,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, Q, F> {
        // SAFETY: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.iter_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
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
        self.validate_world(world);
        // SAFETY: query is read only and world is validated
        unsafe {
            self.as_readonly().iter_unchecked_manual(
                world,
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
        // SAFETY: query is read only
        unsafe {
            self.update_archetypes(world);
            self.as_readonly().iter_combinations_unchecked_manual(
                world,
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
        // SAFETY: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.iter_combinations_unchecked_manual(
                world,
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
                world,
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
        // SAFETY: Query has unique world access.
        unsafe {
            self.iter_many_unchecked_manual(
                entities,
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
    ) -> QueryIter<'w, 's, Q, F> {
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
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
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
    pub(crate) unsafe fn iter_unchecked_manual<'w, 's>(
        &'s self,
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryIter<'w, 's, Q, F> {
        QueryIter::new(world, self, last_change_tick, change_tick)
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
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryManyIter<'w, 's, Q, F, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        QueryManyIter::new(world, self, entities, last_change_tick, change_tick)
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
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> QueryCombinationIter<'w, 's, Q, F, K> {
        QueryCombinationIter::new(world, self, last_change_tick, change_tick)
    }

    /// Runs `func` on each query result for the given [`World`]. This is faster than the equivalent
    /// iter() method, but cannot be chained like a normal [`Iterator`].
    ///
    /// This can only be called for read-only queries, see [`Self::for_each_mut`] for write-queries.
    #[inline]
    pub fn for_each<'w, FN: FnMut(ROQueryItem<'w, Q>)>(&mut self, world: &'w World, func: FN) {
        // SAFETY: query is read only
        unsafe {
            self.update_archetypes(world);
            self.as_readonly().for_each_unchecked_manual(
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
    pub fn for_each_mut<'w, FN: FnMut(Q::Item<'w>)>(&mut self, world: &'w mut World, func: FN) {
        // SAFETY: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.for_each_unchecked_manual(
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
    pub unsafe fn for_each_unchecked<'w, FN: FnMut(Q::Item<'w>)>(
        &mut self,
        world: &'w World,
        func: FN,
    ) {
        self.update_archetypes(world);
        self.for_each_unchecked_manual(
            world,
            func,
            world.last_change_tick(),
            world.read_change_tick(),
        );
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// This can only be called for read-only queries, see [`Self::par_for_each_mut`] for
    /// write-queries.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    #[inline]
    pub fn par_for_each<'w, FN: Fn(ROQueryItem<'w, Q>) + Send + Sync + Clone>(
        &mut self,
        world: &'w World,
        batch_size: usize,
        func: FN,
    ) {
        // SAFETY: query is read only
        unsafe {
            self.update_archetypes(world);
            self.as_readonly().par_for_each_unchecked_manual(
                world,
                batch_size,
                func,
                world.last_change_tick(),
                world.read_change_tick(),
            );
        }
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    #[inline]
    pub fn par_for_each_mut<'w, FN: Fn(Q::Item<'w>) + Send + Sync + Clone>(
        &mut self,
        world: &'w mut World,
        batch_size: usize,
        func: FN,
    ) {
        // SAFETY: query has unique world access
        unsafe {
            self.update_archetypes(world);
            self.par_for_each_unchecked_manual(
                world,
                batch_size,
                func,
                world.last_change_tick(),
                world.read_change_tick(),
            );
        }
    }

    /// Runs `func` on each query result in parallel.
    ///
    /// This can only be called for read-only queries.
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn par_for_each_unchecked<'w, FN: Fn(Q::Item<'w>) + Send + Sync + Clone>(
        &mut self,
        world: &'w World,
        batch_size: usize,
        func: FN,
    ) {
        self.update_archetypes(world);
        self.par_for_each_unchecked_manual(
            world,
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
    pub(crate) unsafe fn for_each_unchecked_manual<'w, FN: FnMut(Q::Item<'w>)>(
        &self,
        world: &'w World,
        mut func: FN,
        last_change_tick: u32,
        change_tick: u32,
    ) {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        let mut fetch = Q::init_fetch(world, &self.fetch_state, last_change_tick, change_tick);
        let mut filter = F::init_fetch(world, &self.filter_state, last_change_tick, change_tick);

        let tables = &world.storages().tables;
        if Q::IS_DENSE && F::IS_DENSE {
            for table_id in &self.matched_table_ids {
                let table = tables.get(*table_id).debug_checked_unwrap();
                Q::set_table(&mut fetch, &self.fetch_state, table);
                F::set_table(&mut filter, &self.filter_state, table);

                let entities = table.entities();
                for row in 0..table.entity_count() {
                    let entity = entities.get_unchecked(row);
                    if !F::filter_fetch(&mut filter, *entity, row) {
                        continue;
                    }
                    func(Q::fetch(&mut fetch, *entity, row));
                }
            }
        } else {
            let archetypes = &world.archetypes;
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
                        archetype_entity.entity,
                        archetype_entity.table_row,
                    ) {
                        continue;
                    }
                    func(Q::fetch(
                        &mut fetch,
                        archetype_entity.entity,
                        archetype_entity.table_row,
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
    pub(crate) unsafe fn par_for_each_unchecked_manual<
        'w,
        FN: Fn(Q::Item<'w>) + Send + Sync + Clone,
    >(
        &self,
        world: &'w World,
        batch_size: usize,
        func: FN,
        last_change_tick: u32,
        change_tick: u32,
    ) {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter, QueryState::for_each_unchecked_manual, QueryState::par_for_each_unchecked_manual
        ComputeTaskPool::get().scope(|scope| {
            if Q::IS_DENSE && F::IS_DENSE {
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
                            let mut fetch = Q::init_fetch(
                                world,
                                &self.fetch_state,
                                last_change_tick,
                                change_tick,
                            );
                            let mut filter = F::init_fetch(
                                world,
                                &self.filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let table = tables.get(*table_id).debug_checked_unwrap();
                            let entities = table.entities();
                            Q::set_table(&mut fetch, &self.fetch_state, table);
                            F::set_table(&mut filter, &self.filter_state, table);
                            for row in offset..offset + len {
                                let entity = entities.get_unchecked(row);
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
                let archetypes = &world.archetypes;
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
                            let mut fetch = Q::init_fetch(
                                world,
                                &self.fetch_state,
                                last_change_tick,
                                change_tick,
                            );
                            let mut filter = F::init_fetch(
                                world,
                                &self.filter_state,
                                last_change_tick,
                                change_tick,
                            );
                            let tables = &world.storages().tables;
                            let archetype =
                                world.archetypes.get(*archetype_id).debug_checked_unwrap();
                            let table = tables.get(archetype.table_id()).debug_checked_unwrap();
                            Q::set_archetype(&mut fetch, &self.fetch_state, archetype, table);
                            F::set_archetype(&mut filter, &self.filter_state, archetype, table);

                            let entities = archetype.entities();
                            for archetype_index in offset..offset + len {
                                let archetype_entity = entities.get_unchecked(archetype_index);
                                if !F::filter_fetch(
                                    &mut filter,
                                    archetype_entity.entity,
                                    archetype_entity.table_row,
                                ) {
                                    continue;
                                }
                                func(Q::fetch(
                                    &mut fetch,
                                    archetype_entity.entity,
                                    archetype_entity.table_row,
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
                world,
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

        // SAFETY: query has unique world access
        unsafe {
            self.get_single_unchecked_manual(
                world,
                world.last_change_tick(),
                world.read_change_tick(),
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
        world: &'w World,
    ) -> Result<Q::Item<'w>, QuerySingleError> {
        self.update_archetypes(world);
        self.get_single_unchecked_manual(world, world.last_change_tick(), world.read_change_tick())
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
        world: &'w World,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Result<Q::Item<'w>, QuerySingleError> {
        let mut query = self.iter_unchecked_manual(world, last_change_tick, change_tick);
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

/// An error that occurs when retrieving a specific [`Entity`]'s query result.
// TODO: return the type_name as part of this error
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum QueryEntityError {
    QueryDoesNotMatch(Entity),
    NoSuchEntity(Entity),
    AliasedMutability(Entity),
}

impl std::error::Error for QueryEntityError {}

impl fmt::Display for QueryEntityError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            QueryEntityError::QueryDoesNotMatch(_) => {
                write!(f, "The given entity does not have the requested component.")
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
        let change_tick = world.read_change_tick();

        // It's best to test get_many_unchecked_manual directly,
        // as it is shared and unsafe
        // We don't care about aliased mutabilty for the read-only equivalent

        // SAFETY: mutable access is not checked, but we own the world and don't use the query results
        assert!(unsafe {
            query_state
                .get_many_unchecked_manual::<10>(
                    &world,
                    entities.clone().try_into().unwrap(),
                    last_change_tick,
                    change_tick,
                )
                .is_ok()
        });

        assert_eq!(
            // SAFETY: mutable access is not checked, but we own the world and don't use the query results
            unsafe {
                query_state
                    .get_many_unchecked_manual(
                        &world,
                        [entities[0], entities[0]],
                        last_change_tick,
                        change_tick,
                    )
                    .unwrap_err()
            },
            QueryEntityError::AliasedMutability(entities[0])
        );

        assert_eq!(
            // SAFETY: mutable access is not checked, but we own the world and don't use the query results
            unsafe {
                query_state
                    .get_many_unchecked_manual(
                        &world,
                        [entities[0], entities[1], entities[0]],
                        last_change_tick,
                        change_tick,
                    )
                    .unwrap_err()
            },
            QueryEntityError::AliasedMutability(entities[0])
        );

        assert_eq!(
            // SAFETY: mutable access is not checked, but we own the world and don't use the query results
            unsafe {
                query_state
                    .get_many_unchecked_manual(
                        &world,
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

/// An error that occurs when evaluating a [`QueryState`] as a single expected resulted via
/// [`QueryState::single`] or [`QueryState::single_mut`].
#[derive(Debug)]
pub enum QuerySingleError {
    NoEntities(&'static str),
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
