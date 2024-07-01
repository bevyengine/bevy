use crate::{
    archetype::{Archetype, ArchetypeComponentId, ArchetypeGeneration, ArchetypeId},
    batching::BatchingStrategy,
    component::{ComponentId, Components, Tick},
    entity::Entity,
    prelude::FromWorld,
    query::{
        Access, DebugCheckedUnwrap, FilteredAccess, QueryCombinationIter, QueryIter, QueryParIter,
    },
    storage::{SparseSetIndex, TableId},
    world::{unsafe_world_cell::UnsafeWorldCell, World, WorldId},
};
use bevy_utils::tracing::warn;
#[cfg(feature = "trace")]
use bevy_utils::tracing::Span;
use fixedbitset::FixedBitSet;
use std::{borrow::Borrow, fmt, mem::MaybeUninit, ptr};

use super::{
    NopWorldQuery, QueryBuilder, QueryData, QueryEntityError, QueryFilter, QueryManyIter,
    QuerySingleError, ROQueryItem,
};

/// An ID for either a table or an archetype. Used for Query iteration.
///
/// Query iteration is exclusively dense (over tables) or archetypal (over archetypes) based on whether
/// both `D::IS_DENSE` and `F::IS_DENSE` are true or not.
///
/// This is a union instead of an enum as the usage is determined at compile time, as all [`StorageId`]s for
/// a [`QueryState`] will be all [`TableId`]s or all [`ArchetypeId`]s, and not a mixture of both. This
/// removes the need for discriminator to minimize memory usage and branching during iteration, but requires
/// a safety invariant be verified when disambiguating them.
///
/// # Safety
/// Must be initialized and accessed as a [`TableId`], if both generic parameters to the query are dense.
/// Must be initialized and accessed as an [`ArchetypeId`] otherwise.
#[derive(Clone, Copy)]
pub(super) union StorageId {
    pub(super) table_id: TableId,
    pub(super) archetype_id: ArchetypeId,
}

/// Provides scoped access to a [`World`] state according to a given [`QueryData`] and [`QueryFilter`].
///
/// This data is cached between system runs, and is used to:
/// - store metadata about which [`Table`] or [`Archetype`] are matched by the query. "Matched" means
///     that the query will iterate over the data in the matched table/archetype.
/// - cache the [`State`] needed to compute the [`Fetch`] struct used to retrieve data
///     from a specific [`Table`] or [`Archetype`]
/// - build iterators that can iterate over the query results
///
/// [`State`]: crate::query::world_query::WorldQuery::State
/// [`Fetch`]: crate::query::world_query::WorldQuery::Fetch
/// [`Table`]: crate::storage::Table
#[repr(C)]
// SAFETY NOTE:
// Do not add any new fields that use the `D` or `F` generic parameters as this may
// make `QueryState::as_transmuted_state` unsound if not done with care.
pub struct QueryState<D: QueryData, F: QueryFilter = ()> {
    world_id: WorldId,
    pub(crate) archetype_generation: ArchetypeGeneration,
    /// Metadata about the [`Table`](crate::storage::Table)s matched by this query.
    pub(crate) matched_tables: FixedBitSet,
    /// Metadata about the [`Archetype`]s matched by this query.
    pub(crate) matched_archetypes: FixedBitSet,
    /// [`FilteredAccess`] computed by combining the `D` and `F` access. Used to check which other queries
    /// this query can run in parallel with.
    pub(crate) component_access: FilteredAccess<ComponentId>,
    // NOTE: we maintain both a bitset and a vec because iterating the vec is faster
    pub(super) matched_storage_ids: Vec<StorageId>,
    pub(crate) fetch_state: D::State,
    pub(crate) filter_state: F::State,
    #[cfg(feature = "trace")]
    par_iter_span: Span,
}

impl<D: QueryData, F: QueryFilter> fmt::Debug for QueryState<D, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("QueryState")
            .field("world_id", &self.world_id)
            .field("matched_table_count", &self.matched_tables.count_ones(..))
            .field(
                "matched_archetype_count",
                &self.matched_archetypes.count_ones(..),
            )
            .finish_non_exhaustive()
    }
}

impl<D: QueryData, F: QueryFilter> FromWorld for QueryState<D, F> {
    fn from_world(world: &mut World) -> Self {
        world.query_filtered()
    }
}

impl<D: QueryData, F: QueryFilter> QueryState<D, F> {
    /// Converts this `QueryState` reference to a `QueryState` that does not access anything mutably.
    pub fn as_readonly(&self) -> &QueryState<D::ReadOnly, F> {
        // SAFETY: invariant on `WorldQuery` trait upholds that `D::ReadOnly` and `F::ReadOnly`
        // have a subset of the access, and match the exact same archetypes/tables as `D`/`F` respectively.
        unsafe { self.as_transmuted_state::<D::ReadOnly, F>() }
    }

    /// Converts this `QueryState` reference to a `QueryState` that does not return any data
    /// which can be faster.
    ///
    /// This doesn't use `NopWorldQuery` as it loses filter functionality, for example
    /// `NopWorldQuery<Changed<T>>` is functionally equivalent to `With<T>`.
    pub(crate) fn as_nop(&self) -> &QueryState<NopWorldQuery<D>, F> {
        // SAFETY: `NopWorldQuery` doesn't have any accesses and defers to
        // `D` for table/archetype matching
        unsafe { self.as_transmuted_state::<NopWorldQuery<D>, F>() }
    }

    /// Converts this `QueryState` reference to any other `QueryState` with
    /// the same `WorldQuery::State` associated types.
    ///
    /// Consider using `as_readonly` or `as_nop` instead which are safe functions.
    ///
    /// # SAFETY
    ///
    /// `NewD` must have a subset of the access that `D` does and match the exact same archetypes/tables
    /// `NewF` must have a subset of the access that `F` does and match the exact same archetypes/tables
    pub(crate) unsafe fn as_transmuted_state<
        NewD: QueryData<State = D::State>,
        NewF: QueryFilter<State = F::State>,
    >(
        &self,
    ) -> &QueryState<NewD, NewF> {
        &*ptr::from_ref(self).cast::<QueryState<NewD, NewF>>()
    }

    /// Returns the components accessed by this query.
    pub fn component_access(&self) -> &FilteredAccess<ComponentId> {
        &self.component_access
    }

    /// Returns the tables matched by this query.
    pub fn matched_tables(&self) -> impl Iterator<Item = TableId> + '_ {
        self.matched_tables.ones().map(TableId::from_usize)
    }

    /// Returns the archetypes matched by this query.
    pub fn matched_archetypes(&self) -> impl Iterator<Item = ArchetypeId> + '_ {
        self.matched_archetypes.ones().map(ArchetypeId::new)
    }
}

impl<D: QueryData, F: QueryFilter> QueryState<D, F> {
    /// Creates a new [`QueryState`] from a given [`World`] and inherits the result of `world.id()`.
    pub fn new(world: &mut World) -> Self {
        let mut state = Self::new_uninitialized(world);
        state.update_archetypes(world);
        state
    }

    /// Identical to `new`, but it populates the provided `access` with the matched results.
    pub(crate) fn new_with_access(
        world: &mut World,
        access: &mut Access<ArchetypeComponentId>,
    ) -> Self {
        let mut state = Self::new_uninitialized(world);
        for archetype in world.archetypes.iter() {
            // SAFETY: The state was just initialized from the `world` above, and the archetypes being added
            // come directly from the same world.
            unsafe {
                if state.new_archetype_internal(archetype) {
                    state.update_archetype_component_access(archetype, access);
                }
            }
        }
        state.archetype_generation = world.archetypes.generation();
        state
    }

    /// Creates a new [`QueryState`] but does not populate it with the matched results from the World yet
    ///
    /// `new_archetype` and its variants must be called on all of the World's archetypes before the
    /// state can return valid query results.
    fn new_uninitialized(world: &mut World) -> Self {
        let fetch_state = D::init_state(world);
        let filter_state = F::init_state(world);

        let mut component_access = FilteredAccess::default();
        D::update_component_access(&fetch_state, &mut component_access);

        // Use a temporary empty FilteredAccess for filters. This prevents them from conflicting with the
        // main Query's `fetch_state` access. Filters are allowed to conflict with the main query fetch
        // because they are evaluated *before* a specific reference is constructed.
        let mut filter_component_access = FilteredAccess::default();
        F::update_component_access(&filter_state, &mut filter_component_access);

        // Merge the temporary filter access with the main access. This ensures that filter access is
        // properly considered in a global "cross-query" context (both within systems and across systems).
        component_access.extend(&filter_component_access);

        Self {
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
            matched_storage_ids: Vec::new(),
            fetch_state,
            filter_state,
            component_access,
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            #[cfg(feature = "trace")]
            par_iter_span: bevy_utils::tracing::info_span!(
                "par_for_each",
                query = std::any::type_name::<D>(),
                filter = std::any::type_name::<F>(),
            ),
        }
    }

    /// Creates a new [`QueryState`] from a given [`QueryBuilder`] and inherits its [`FilteredAccess`].
    pub fn from_builder(builder: &mut QueryBuilder<D, F>) -> Self {
        let mut fetch_state = D::init_state(builder.world_mut());
        let filter_state = F::init_state(builder.world_mut());
        D::set_access(&mut fetch_state, builder.access());

        let mut state = Self {
            world_id: builder.world().id(),
            archetype_generation: ArchetypeGeneration::initial(),
            matched_storage_ids: Vec::new(),
            fetch_state,
            filter_state,
            component_access: builder.access().clone(),
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            #[cfg(feature = "trace")]
            par_iter_span: bevy_utils::tracing::info_span!(
                "par_for_each",
                data = std::any::type_name::<D>(),
                filter = std::any::type_name::<F>(),
            ),
        };
        state.update_archetypes(builder.world());
        state
    }

    /// Checks if the query is empty for the given [`World`], where the last change and current tick are given.
    ///
    /// This is equivalent to `self.iter().next().is_none()`, and thus the worst case runtime will be `O(n)`
    /// where `n` is the number of *potential* matches. This can be notably expensive for queries that rely
    /// on non-archetypal filters such as [`Added`] or [`Changed`] which must individually check each query
    /// result for a match.
    ///
    /// # Panics
    ///
    /// If `world` does not match the one used to call `QueryState::new` for this instance.
    ///
    /// [`Added`]: crate::query::Added
    /// [`Changed`]: crate::query::Changed
    #[inline]
    pub fn is_empty(&self, world: &World, last_run: Tick, this_run: Tick) -> bool {
        self.validate_world(world.id());
        // SAFETY:
        // - We have read-only access to the entire world.
        // - The world has been validated.
        unsafe {
            self.is_empty_unsafe_world_cell(
                world.as_unsafe_world_cell_readonly(),
                last_run,
                this_run,
            )
        }
    }

    /// Returns `true` if the given [`Entity`] matches the query.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn contains(&self, entity: Entity, world: &World, last_run: Tick, this_run: Tick) -> bool {
        // SAFETY: NopFetch does not access any members while &self ensures no one has exclusive access
        unsafe {
            self.as_nop()
                .get_unchecked_manual(
                    world.as_unsafe_world_cell_readonly(),
                    entity,
                    last_run,
                    this_run,
                )
                .is_ok()
        }
    }

    /// Checks if the query is empty for the given [`UnsafeWorldCell`].
    ///
    /// # Safety
    ///
    /// - `world` must have permission to read any components required by this instance's `F` [`QueryFilter`].
    /// - `world` must match the one used to create this [`QueryState`].
    #[inline]
    pub(crate) unsafe fn is_empty_unsafe_world_cell(
        &self,
        world: UnsafeWorldCell,
        last_run: Tick,
        this_run: Tick,
    ) -> bool {
        // SAFETY:
        // - The caller ensures that `world` has permission to access any data used by the filter.
        // - The caller ensures that the world matches.
        unsafe {
            self.as_nop()
                .iter_unchecked_manual(world, last_run, this_run)
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
        let old_generation =
            std::mem::replace(&mut self.archetype_generation, archetypes.generation());

        for archetype in &archetypes[old_generation..] {
            // SAFETY: The validate_world call ensures that the world is the same the QueryState
            // was initialized from.
            unsafe {
                self.new_archetype_internal(archetype);
            }
        }
    }

    /// # Panics
    ///
    /// If `world_id` does not match the [`World`] used to call `QueryState::new` for this instance.
    ///
    /// Many unsafe query methods require the world to match for soundness. This function is the easiest
    /// way of ensuring that it matches.
    #[inline]
    #[track_caller]
    pub fn validate_world(&self, world_id: WorldId) {
        #[inline(never)]
        #[track_caller]
        #[cold]
        fn panic_mismatched(this: WorldId, other: WorldId) -> ! {
            panic!("Encountered a mismatched World. This QueryState was created from {this:?}, but a method was called using {other:?}.");
        }

        if self.world_id != world_id {
            panic_mismatched(self.world_id, world_id);
        }
    }

    /// Update the current [`QueryState`] with information from the provided [`Archetype`]
    /// (if applicable, i.e. if the archetype has any intersecting [`ComponentId`] with the current [`QueryState`]).
    ///
    /// The passed in `access` will be updated with any new accesses introduced by the new archetype.
    ///
    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    pub unsafe fn new_archetype(
        &mut self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from.
        let matches = unsafe { self.new_archetype_internal(archetype) };
        if matches {
            // SAFETY: The caller ensures that `archetype` is from the World the state was initialized from.
            unsafe { self.update_archetype_component_access(archetype, access) };
        }
    }

    /// Process the given [`Archetype`] to update internal metadata about the [`Table`](crate::storage::Table)s
    /// and [`Archetype`]s that are matched by this query.
    ///
    /// Returns `true` if the given `archetype` matches the query. Otherwise, returns `false`.
    /// If there is no match, then there is no need to update the query's [`FilteredAccess`].
    ///
    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    unsafe fn new_archetype_internal(&mut self, archetype: &Archetype) -> bool {
        if D::matches_component_set(&self.fetch_state, &|id| archetype.contains(id))
            && F::matches_component_set(&self.filter_state, &|id| archetype.contains(id))
            && self.matches_component_set(&|id| archetype.contains(id))
        {
            let archetype_index = archetype.id().index();
            if !self.matched_archetypes.contains(archetype_index) {
                self.matched_archetypes.grow_and_insert(archetype_index);
                if !D::IS_DENSE || !F::IS_DENSE {
                    self.matched_storage_ids.push(StorageId {
                        archetype_id: archetype.id(),
                    });
                }
            }
            let table_index = archetype.table_id().as_usize();
            if !self.matched_tables.contains(table_index) {
                self.matched_tables.grow_and_insert(table_index);
                if D::IS_DENSE && F::IS_DENSE {
                    self.matched_storage_ids.push(StorageId {
                        table_id: archetype.table_id(),
                    });
                }
            }
            true
        } else {
            false
        }
    }

    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    pub fn matches_component_set(&self, set_contains_id: &impl Fn(ComponentId) -> bool) -> bool {
        self.component_access.filter_sets.iter().any(|set| {
            set.with
                .ones()
                .all(|index| set_contains_id(ComponentId::get_sparse_set_index(index)))
                && set
                    .without
                    .ones()
                    .all(|index| !set_contains_id(ComponentId::get_sparse_set_index(index)))
        })
    }

    /// For the given `archetype`, adds any component accessed used by this query's underlying [`FilteredAccess`] to `access`.
    ///
    /// The passed in `access` will be updated with any new accesses introduced by the new archetype.
    ///
    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    pub unsafe fn update_archetype_component_access(
        &mut self,
        archetype: &Archetype,
        access: &mut Access<ArchetypeComponentId>,
    ) {
        self.component_access.access.reads().for_each(|id| {
            if let Some(id) = archetype.get_archetype_component_id(id) {
                access.add_read(id);
            }
        });
        self.component_access.access.writes().for_each(|id| {
            if let Some(id) = archetype.get_archetype_component_id(id) {
                access.add_write(id);
            }
        });
    }

    /// Use this to transform a [`QueryState`] into a more generic [`QueryState`].
    /// This can be useful for passing to another function that might take the more general form.
    /// See [`Query::transmute_lens`](crate::system::Query::transmute_lens) for more details.
    ///
    /// You should not call [`update_archetypes`](Self::update_archetypes) on the returned [`QueryState`] as the result will be unpredictable.
    /// You might end up with a mix of archetypes that only matched the original query + archetypes that only match
    /// the new [`QueryState`]. Most of the safe methods on [`QueryState`] call [`QueryState::update_archetypes`] internally, so this
    /// best used through a [`Query`](crate::system::Query).
    pub fn transmute<NewD: QueryData>(&self, components: &Components) -> QueryState<NewD> {
        self.transmute_filtered::<NewD, ()>(components)
    }

    /// Creates a new [`QueryState`] with the same underlying [`FilteredAccess`], matched tables and archetypes
    /// as self but with a new type signature.
    ///
    /// Panics if `NewD` or `NewF` require accesses that this query does not have.
    pub fn transmute_filtered<NewD: QueryData, NewF: QueryFilter>(
        &self,
        components: &Components,
    ) -> QueryState<NewD, NewF> {
        let mut component_access = FilteredAccess::default();
        let mut fetch_state = NewD::get_state(components).expect("Could not create fetch_state, Please initialize all referenced components before transmuting.");
        let filter_state = NewF::get_state(components).expect("Could not create filter_state, Please initialize all referenced components before transmuting.");

        NewD::set_access(&mut fetch_state, &self.component_access);
        NewD::update_component_access(&fetch_state, &mut component_access);

        let mut filter_component_access = FilteredAccess::default();
        NewF::update_component_access(&filter_state, &mut filter_component_access);

        component_access.extend(&filter_component_access);
        assert!(
            component_access.is_subset(&self.component_access),
            "Transmuted state for {} attempts to access terms that are not allowed by original state {}.",
            std::any::type_name::<(NewD, NewF)>(), std::any::type_name::<(D, F)>()
        );

        QueryState {
            world_id: self.world_id,
            archetype_generation: self.archetype_generation,
            matched_storage_ids: self.matched_storage_ids.clone(),
            fetch_state,
            filter_state,
            component_access: self.component_access.clone(),
            matched_tables: self.matched_tables.clone(),
            matched_archetypes: self.matched_archetypes.clone(),
            #[cfg(feature = "trace")]
            par_iter_span: bevy_utils::tracing::info_span!(
                "par_for_each",
                query = std::any::type_name::<NewD>(),
                filter = std::any::type_name::<NewF>(),
            ),
        }
    }

    /// Use this to combine two queries. The data accessed will be the intersection
    /// of archetypes included in both queries. This can be useful for accessing a
    /// subset of the entities between two queries.
    ///
    /// You should not call `update_archetypes` on the returned `QueryState` as the result
    /// could be unpredictable. You might end up with a mix of archetypes that only matched
    /// the original query + archetypes that only match the new `QueryState`. Most of the
    /// safe methods on `QueryState` call [`QueryState::update_archetypes`] internally, so
    /// this is best used through a `Query`.
    ///
    /// ## Performance
    ///
    /// This will have similar performance as constructing a new `QueryState` since much of internal state
    /// needs to be reconstructed. But it will be a little faster as it only needs to compare the intersection
    /// of matching archetypes rather than iterating over all archetypes.
    ///
    /// ## Panics
    ///
    /// Will panic if `NewD` contains accesses not in `Q` or `OtherQ`.
    pub fn join<OtherD: QueryData, NewD: QueryData>(
        &self,
        components: &Components,
        other: &QueryState<OtherD>,
    ) -> QueryState<NewD, ()> {
        self.join_filtered::<_, (), NewD, ()>(components, other)
    }

    /// Use this to combine two queries. The data accessed will be the intersection
    /// of archetypes included in both queries.
    ///
    /// ## Panics
    ///
    /// Will panic if `NewD` or `NewF` requires accesses not in `Q` or `OtherQ`.
    pub fn join_filtered<
        OtherD: QueryData,
        OtherF: QueryFilter,
        NewD: QueryData,
        NewF: QueryFilter,
    >(
        &self,
        components: &Components,
        other: &QueryState<OtherD, OtherF>,
    ) -> QueryState<NewD, NewF> {
        if self.world_id != other.world_id {
            panic!("Joining queries initialized on different worlds is not allowed.");
        }

        let mut component_access = FilteredAccess::default();
        let mut new_fetch_state = NewD::get_state(components)
            .expect("Could not create fetch_state, Please initialize all referenced components before transmuting.");
        let new_filter_state = NewF::get_state(components)
            .expect("Could not create filter_state, Please initialize all referenced components before transmuting.");

        NewD::set_access(&mut new_fetch_state, &self.component_access);
        NewD::update_component_access(&new_fetch_state, &mut component_access);

        let mut new_filter_component_access = FilteredAccess::default();
        NewF::update_component_access(&new_filter_state, &mut new_filter_component_access);

        component_access.extend(&new_filter_component_access);

        let mut joined_component_access = self.component_access.clone();
        joined_component_access.extend(&other.component_access);

        assert!(
            component_access.is_subset(&joined_component_access),
            "Joined state for {} attempts to access terms that are not allowed by state {} joined with {}.",
            std::any::type_name::<(NewD, NewF)>(), std::any::type_name::<(D, F)>(), std::any::type_name::<(OtherD, OtherF)>()
        );

        if self.archetype_generation != other.archetype_generation {
            warn!("You have tried to join queries with different archetype_generations. This could lead to unpredictable results.");
        }

        // take the intersection of the matched ids
        let mut matched_tables = self.matched_tables.clone();
        let mut matched_archetypes = self.matched_archetypes.clone();
        matched_tables.intersect_with(&other.matched_tables);
        matched_archetypes.intersect_with(&other.matched_archetypes);
        let matched_storage_ids = if NewD::IS_DENSE && NewF::IS_DENSE {
            matched_tables
                .ones()
                .map(|id| StorageId {
                    table_id: TableId::from_usize(id),
                })
                .collect()
        } else {
            matched_archetypes
                .ones()
                .map(|id| StorageId {
                    archetype_id: ArchetypeId::new(id),
                })
                .collect()
        };

        QueryState {
            world_id: self.world_id,
            archetype_generation: self.archetype_generation,
            matched_storage_ids,
            fetch_state: new_fetch_state,
            filter_state: new_filter_state,
            component_access: joined_component_access,
            matched_tables,
            matched_archetypes,
            #[cfg(feature = "trace")]
            par_iter_span: bevy_utils::tracing::info_span!(
                "par_for_each",
                query = std::any::type_name::<NewD>(),
                filter = std::any::type_name::<NewF>(),
            ),
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for write-queries.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, D>, QueryEntityError> {
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
    /// ```
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
    ) -> Result<[ROQueryItem<'w, D>; N], QueryEntityError> {
        self.update_archetypes(world);

        // SAFETY:
        // - We have read-only access to the entire world.
        // - `update_archetypes` validates that the `World` matches.
        unsafe {
            self.get_many_read_only_manual(
                world.as_unsafe_world_cell_readonly(),
                entities,
                world.last_change_tick(),
                world.read_change_tick(),
            )
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get_mut<'w>(
        &mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Result<D::Item<'w>, QueryEntityError> {
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
    /// ```
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
    ) -> Result<[D::Item<'w>; N], QueryEntityError> {
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
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get_manual<'w>(
        &self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, D>, QueryEntityError> {
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
    /// This is always guaranteed to run in `O(1)` time.
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
    ) -> Result<D::Item<'w>, QueryEntityError> {
        self.update_archetypes_unsafe_world_cell(world);
        self.get_unchecked_manual(world, entity, world.last_change_tick(), world.change_tick())
    }

    /// Gets the query result for the given [`World`] and [`Entity`], where the last change and
    /// the current change tick are given.
    ///
    /// This is always guaranteed to run in `O(1)` time.
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
    ) -> Result<D::Item<'w>, QueryEntityError> {
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
        let mut fetch = D::init_fetch(world, &self.fetch_state, last_run, this_run);
        let mut filter = F::init_fetch(world, &self.filter_state, last_run, this_run);

        let table = world
            .storages()
            .tables
            .get(location.table_id)
            .debug_checked_unwrap();
        D::set_archetype(&mut fetch, &self.fetch_state, archetype, table);
        F::set_archetype(&mut filter, &self.filter_state, archetype, table);

        if F::filter_fetch(&mut filter, entity, location.table_row) {
            Ok(D::fetch(&mut fetch, entity, location.table_row))
        } else {
            Err(QueryEntityError::QueryDoesNotMatch(entity))
        }
    }

    /// Gets the read-only query results for the given [`World`] and array of [`Entity`], where the last change and
    /// the current change tick are given.
    ///
    /// # Safety
    ///
    /// * `world` must have permission to read all of the components returned from this call.
    ///     No mutable references may coexist with any of the returned references.
    /// * This must be called on the same `World` that the `Query` was generated from:
    ///     use `QueryState::validate_world` to verify this.
    pub(crate) unsafe fn get_many_read_only_manual<'w, const N: usize>(
        &self,
        world: UnsafeWorldCell<'w>,
        entities: [Entity; N],
        last_run: Tick,
        this_run: Tick,
    ) -> Result<[ROQueryItem<'w, D>; N], QueryEntityError> {
        let mut values = [(); N].map(|_| MaybeUninit::uninit());

        for (value, entity) in std::iter::zip(&mut values, entities) {
            // SAFETY: fetch is read-only and world must be validated
            let item = unsafe {
                self.as_readonly()
                    .get_unchecked_manual(world, entity, last_run, this_run)?
            };
            *value = MaybeUninit::new(item);
        }

        // SAFETY: Each value has been fully initialized.
        Ok(values.map(|x| unsafe { x.assume_init() }))
    }

    /// Gets the query results for the given [`World`] and array of [`Entity`], where the last change and
    /// the current change tick are given.
    ///
    /// This is always guaranteed to run in `O(1)` time.
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
    ) -> Result<[D::Item<'w>; N], QueryEntityError> {
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
    pub fn iter<'w, 's>(&'s mut self, world: &'w World) -> QueryIter<'w, 's, D::ReadOnly, F> {
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
    ///
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, D, F> {
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
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    ///
    /// This can only be called for read-only queries.
    #[inline]
    pub fn iter_manual<'w, 's>(&'s self, world: &'w World) -> QueryIter<'w, 's, D::ReadOnly, F> {
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
    /// This iterator is always guaranteed to return results from each unique pair of matching entities.
    /// Iteration order is not guaranteed.
    ///
    /// This can only be called for read-only queries, see [`Self::iter_combinations_mut`] for
    /// write-queries.
    #[inline]
    pub fn iter_combinations<'w, 's, const K: usize>(
        &'s mut self,
        world: &'w World,
    ) -> QueryCombinationIter<'w, 's, D::ReadOnly, F, K> {
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
    ) -> QueryCombinationIter<'w, 's, D, F, K> {
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
    ) -> QueryManyIter<'w, 's, D::ReadOnly, F, EntityList::IntoIter>
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
    ) -> QueryManyIter<'w, 's, D::ReadOnly, F, EntityList::IntoIter>
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
    ) -> QueryManyIter<'w, 's, D, F, EntityList::IntoIter>
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
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_unchecked<'w, 's>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
    ) -> QueryIter<'w, 's, D, F> {
        self.update_archetypes_unsafe_world_cell(world);
        self.iter_unchecked_manual(world, world.last_change_tick(), world.change_tick())
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results for the
    /// given [`World`] without repetition.
    /// This can only be called for read-only queries.
    ///
    /// This iterator is always guaranteed to return results from each unique pair of matching entities.
    /// Iteration order is not guaranteed.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    #[inline]
    pub unsafe fn iter_combinations_unchecked<'w, 's, const K: usize>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
    ) -> QueryCombinationIter<'w, 's, D, F, K> {
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
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
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
    ) -> QueryIter<'w, 's, D, F> {
        QueryIter::new(world, self, last_run, this_run)
    }

    /// Returns an [`Iterator`] for the given [`World`] and list of [`Entity`]'s, where the last change and
    /// the current change tick are given.
    ///
    /// This iterator is always guaranteed to return results from each unique pair of matching entities.
    /// Iteration order is not guaranteed.
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
    ) -> QueryManyIter<'w, 's, D, F, EntityList::IntoIter>
    where
        EntityList::Item: Borrow<Entity>,
    {
        QueryManyIter::new(world, self, entities, last_run, this_run)
    }

    /// Returns an [`Iterator`] over all possible combinations of `K` query results for the
    /// given [`World`] without repetition.
    /// This can only be called for read-only queries.
    ///
    /// This iterator is always guaranteed to return results from each unique pair of matching entities.
    /// Iteration order is not guaranteed.
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
    ) -> QueryCombinationIter<'w, 's, D, F, K> {
        QueryCombinationIter::new(world, self, last_run, this_run)
    }

    /// Returns a parallel iterator over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`par_iter_mut`] for write-queries.
    ///
    /// Note that you must use the `for_each` method to iterate over the
    /// results, see [`par_iter_mut`] for an example.
    ///
    /// [`par_iter_mut`]: Self::par_iter_mut
    #[inline]
    pub fn par_iter<'w, 's>(
        &'s mut self,
        world: &'w World,
    ) -> QueryParIter<'w, 's, D::ReadOnly, F> {
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
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::query::QueryEntityError;
    ///
    /// #[derive(Component, PartialEq, Debug)]
    /// struct A(usize);
    ///
    /// # bevy_tasks::ComputeTaskPool::get_or_init(|| bevy_tasks::TaskPool::new());
    ///
    /// let mut world = World::new();
    ///
    /// # let entities: Vec<Entity> = (0..3).map(|i| world.spawn(A(i)).id()).collect();
    /// # let entities: [Entity; 3] = entities.try_into().unwrap();
    ///
    /// let mut query_state = world.query::<&mut A>();
    ///
    /// query_state.par_iter_mut(&mut world).for_each(|mut a| {
    ///     a.0 += 5;
    /// });
    ///
    /// # let component_values = query_state.get_many(&world, entities).unwrap();
    ///
    /// # assert_eq!(component_values, [&A(5), &A(6), &A(7)]);
    ///
    /// # let wrong_entity = Entity::from_raw(57);
    /// # let invalid_entity = world.spawn_empty().id();
    ///
    /// # assert_eq!(query_state.get_many_mut(&mut world, [wrong_entity]).unwrap_err(), QueryEntityError::NoSuchEntity(wrong_entity));
    /// # assert_eq!(query_state.get_many_mut(&mut world, [invalid_entity]).unwrap_err(), QueryEntityError::QueryDoesNotMatch(invalid_entity));
    /// # assert_eq!(query_state.get_many_mut(&mut world, [entities[0], entities[0]]).unwrap_err(), QueryEntityError::AliasedMutability(entities[0]));
    /// ```
    ///
    /// # Panics
    /// The [`ComputeTaskPool`] is not initialized. If using this from a query that is being
    /// initialized and run from the ECS scheduler, this should never panic.
    ///
    /// [`par_iter`]: Self::par_iter
    /// [`ComputeTaskPool`]: bevy_tasks::ComputeTaskPool
    #[inline]
    pub fn par_iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryParIter<'w, 's, D, F> {
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

    /// Runs `func` on each query result in parallel for the given [`World`], where the last change and
    /// the current change tick are given. This is faster than the equivalent
    /// `iter()` method, but cannot be chained like a normal [`Iterator`].
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
    #[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
    pub(crate) unsafe fn par_fold_init_unchecked_manual<'w, T, FN, INIT>(
        &self,
        init_accum: INIT,
        world: UnsafeWorldCell<'w>,
        batch_size: usize,
        func: FN,
        last_run: Tick,
        this_run: Tick,
    ) where
        FN: Fn(T, D::Item<'w>) -> T + Send + Sync + Clone,
        INIT: Fn() -> T + Sync + Send + Clone,
    {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter,QueryState::par_fold_init_unchecked_manual
        use arrayvec::ArrayVec;

        bevy_tasks::ComputeTaskPool::get().scope(|scope| {
            // SAFETY: We only access table data that has been registered in `self.archetype_component_access`.
            let tables = unsafe { &world.storages().tables };
            let archetypes = world.archetypes();
            let mut batch_queue = ArrayVec::new();
            let mut queue_entity_count = 0;

            // submit a list of storages which smaller than batch_size as single task
            let submit_batch_queue = |queue: &mut ArrayVec<StorageId, 128>| {
                if queue.is_empty() {
                    return;
                }
                let queue = std::mem::take(queue);
                let mut func = func.clone();
                let init_accum = init_accum.clone();
                scope.spawn(async move {
                    #[cfg(feature = "trace")]
                    let _span = self.par_iter_span.enter();
                    let mut iter = self.iter_unchecked_manual(world, last_run, this_run);
                    let mut accum = init_accum();
                    for storage_id in queue {
                        if D::IS_DENSE && F::IS_DENSE {
                            let id = storage_id.table_id;
                            let table = &world.storages().tables.get(id).debug_checked_unwrap();
                            accum = iter.fold_over_table_range(
                                accum,
                                &mut func,
                                table,
                                0..table.entity_count(),
                            );
                        } else {
                            let id = storage_id.archetype_id;
                            let archetype = world.archetypes().get(id).debug_checked_unwrap();
                            accum = iter.fold_over_archetype_range(
                                accum,
                                &mut func,
                                archetype,
                                0..archetype.len(),
                            );
                        }
                    }
                });
            };

            // submit single storage larger than batch_size
            let submit_single = |count, storage_id: StorageId| {
                for offset in (0..count).step_by(batch_size) {
                    let mut func = func.clone();
                    let init_accum = init_accum.clone();
                    let len = batch_size.min(count - offset);
                    let batch = offset..offset + len;
                    scope.spawn(async move {
                        #[cfg(feature = "trace")]
                        let _span = self.par_iter_span.enter();
                        let accum = init_accum();
                        if D::IS_DENSE && F::IS_DENSE {
                            let id = storage_id.table_id;
                            let table = world.storages().tables.get(id).debug_checked_unwrap();
                            self.iter_unchecked_manual(world, last_run, this_run)
                                .fold_over_table_range(accum, &mut func, table, batch);
                        } else {
                            let id = storage_id.archetype_id;
                            let archetype = world.archetypes().get(id).debug_checked_unwrap();
                            self.iter_unchecked_manual(world, last_run, this_run)
                                .fold_over_archetype_range(accum, &mut func, archetype, batch);
                        }
                    });
                }
            };

            let storage_entity_count = |storage_id: StorageId| -> usize {
                if D::IS_DENSE && F::IS_DENSE {
                    tables[storage_id.table_id].entity_count()
                } else {
                    archetypes[storage_id.archetype_id].len()
                }
            };

            for storage_id in &self.matched_storage_ids {
                let count = storage_entity_count(*storage_id);

                // skip empty storage
                if count == 0 {
                    continue;
                }
                // immediately submit large storage
                if count >= batch_size {
                    submit_single(count, *storage_id);
                    continue;
                }
                // merge small storage
                batch_queue.push(*storage_id);
                queue_entity_count += count;

                // submit batch_queue
                if queue_entity_count >= batch_size || batch_queue.is_full() {
                    submit_batch_queue(&mut batch_queue);
                    queue_entity_count = 0;
                }
            }
            submit_batch_queue(&mut batch_queue);
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
    pub fn single<'w>(&mut self, world: &'w World) -> ROQueryItem<'w, D> {
        match self.get_single(world) {
            Ok(items) => items,
            Err(error) => panic!("Cannot get single mutable query result: {error}"),
        }
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
    ) -> Result<ROQueryItem<'w, D>, QuerySingleError> {
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
    pub fn single_mut<'w>(&mut self, world: &'w mut World) -> D::Item<'w> {
        // SAFETY: query has unique world access
        match self.get_single_mut(world) {
            Ok(items) => items,
            Err(error) => panic!("Cannot get single query result: {error}"),
        }
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
    ) -> Result<D::Item<'w>, QuerySingleError> {
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
    ) -> Result<D::Item<'w>, QuerySingleError> {
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
    ) -> Result<D::Item<'w>, QuerySingleError> {
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

impl<D: QueryData, F: QueryFilter> From<QueryBuilder<'_, D, F>> for QueryState<D, F> {
    fn from(mut value: QueryBuilder<D, F>) -> Self {
        QueryState::from_builder(&mut value)
    }
}

#[cfg(test)]
mod tests {
    use crate as bevy_ecs;
    use crate::world::FilteredEntityRef;
    use crate::{component::Component, prelude::*, query::QueryEntityError};

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

    #[derive(Component, PartialEq, Debug)]
    struct A(usize);

    #[derive(Component, PartialEq, Debug)]
    struct B(usize);

    #[derive(Component, PartialEq, Debug)]
    struct C(usize);

    #[test]
    fn can_transmute_to_more_general() {
        let mut world = World::new();
        world.spawn((A(1), B(0)));

        let query_state = world.query::<(&A, &B)>();
        let mut new_query_state = query_state.transmute::<&A>(world.components());
        assert_eq!(new_query_state.iter(&world).len(), 1);
        let a = new_query_state.single(&world);

        assert_eq!(a.0, 1);
    }

    #[test]
    fn cannot_get_data_not_in_original_query() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));
        world.spawn((A(1), B(0), C(0)));

        let query_state = world.query_filtered::<(&A, &B), Without<C>>();
        let mut new_query_state = query_state.transmute::<&A>(world.components());
        // even though we change the query to not have Without<C>, we do not get the component with C.
        let a = new_query_state.single(&world);

        assert_eq!(a.0, 0);
    }

    #[test]
    fn can_transmute_empty_tuple() {
        let mut world = World::new();
        world.init_component::<A>();
        let entity = world.spawn(A(10)).id();

        let q = world.query::<()>();
        let mut q = q.transmute::<Entity>(world.components());
        assert_eq!(q.single(&world), entity);
    }

    #[test]
    fn can_transmute_immut_fetch() {
        let mut world = World::new();
        world.spawn(A(10));

        let q = world.query::<&A>();
        let mut new_q = q.transmute::<Ref<A>>(world.components());
        assert!(new_q.single(&world).is_added());

        let q = world.query::<Ref<A>>();
        let _ = q.transmute::<&A>(world.components());
    }

    #[test]
    fn can_transmute_mut_fetch() {
        let mut world = World::new();
        world.spawn(A(0));

        let q = world.query::<&mut A>();
        let _ = q.transmute::<Ref<A>>(world.components());
        let _ = q.transmute::<&A>(world.components());
    }

    #[test]
    fn can_transmute_entity_mut() {
        let mut world = World::new();
        world.spawn(A(0));

        let q: QueryState<EntityMut<'_>> = world.query::<EntityMut>();
        let _ = q.transmute::<EntityRef>(world.components());
    }

    #[test]
    fn can_generalize_with_option() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));

        let query_state = world.query::<(Option<&A>, &B)>();
        let _ = query_state.transmute::<Option<&A>>(world.components());
        let _ = query_state.transmute::<&B>(world.components());
    }

    #[test]
    #[should_panic(
        expected = "Transmuted state for ((&bevy_ecs::query::state::tests::A, &bevy_ecs::query::state::tests::B), ()) attempts to access terms that are not allowed by original state (&bevy_ecs::query::state::tests::A, ())."
    )]
    fn cannot_transmute_to_include_data_not_in_original_query() {
        let mut world = World::new();
        world.init_component::<A>();
        world.init_component::<B>();
        world.spawn(A(0));

        let query_state = world.query::<&A>();
        let mut _new_query_state = query_state.transmute::<(&A, &B)>(world.components());
    }

    #[test]
    #[should_panic(
        expected = "Transmuted state for (&mut bevy_ecs::query::state::tests::A, ()) attempts to access terms that are not allowed by original state (&bevy_ecs::query::state::tests::A, ())."
    )]
    fn cannot_transmute_immut_to_mut() {
        let mut world = World::new();
        world.spawn(A(0));

        let query_state = world.query::<&A>();
        let mut _new_query_state = query_state.transmute::<&mut A>(world.components());
    }

    #[test]
    #[should_panic(
        expected = "Transmuted state for (&bevy_ecs::query::state::tests::A, ()) attempts to access terms that are not allowed by original state (core::option::Option<&bevy_ecs::query::state::tests::A>, ())."
    )]
    fn cannot_transmute_option_to_immut() {
        let mut world = World::new();
        world.spawn(C(0));

        let query_state = world.query::<Option<&A>>();
        let mut new_query_state = query_state.transmute::<&A>(world.components());
        let x = new_query_state.single(&world);
        assert_eq!(x.0, 1234);
    }

    #[test]
    #[should_panic(
        expected = "Transmuted state for (&bevy_ecs::query::state::tests::A, ()) attempts to access terms that are not allowed by original state (bevy_ecs::world::entity_ref::EntityRef, ())."
    )]
    fn cannot_transmute_entity_ref() {
        let mut world = World::new();
        world.init_component::<A>();

        let q = world.query::<EntityRef>();
        let _ = q.transmute::<&A>(world.components());
    }

    #[test]
    fn can_transmute_filtered_entity() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();
        let query = QueryState::<(Entity, &A, &B)>::new(&mut world)
            .transmute::<FilteredEntityRef>(world.components());

        let mut query = query;
        // Our result is completely untyped
        let entity_ref = query.single(&world);

        assert_eq!(entity, entity_ref.id());
        assert_eq!(0, entity_ref.get::<A>().unwrap().0);
        assert_eq!(1, entity_ref.get::<B>().unwrap().0);
    }

    #[test]
    fn can_transmute_added() {
        let mut world = World::new();
        let entity_a = world.spawn(A(0)).id();

        let mut query = QueryState::<(Entity, &A, Has<B>)>::new(&mut world)
            .transmute_filtered::<(Entity, Has<B>), Added<A>>(world.components());

        assert_eq!((entity_a, false), query.single(&world));

        world.clear_trackers();

        let entity_b = world.spawn((A(0), B(0))).id();
        assert_eq!((entity_b, true), query.single(&world));

        world.clear_trackers();

        assert!(query.get_single(&world).is_err());
    }

    #[test]
    fn can_transmute_changed() {
        let mut world = World::new();
        let entity_a = world.spawn(A(0)).id();

        let mut detection_query = QueryState::<(Entity, &A)>::new(&mut world)
            .transmute_filtered::<Entity, Changed<A>>(world.components());

        let mut change_query = QueryState::<&mut A>::new(&mut world);
        assert_eq!(entity_a, detection_query.single(&world));

        world.clear_trackers();

        assert!(detection_query.get_single(&world).is_err());

        change_query.single_mut(&mut world).0 = 1;

        assert_eq!(entity_a, detection_query.single(&world));
    }

    #[test]
    #[should_panic(
        expected = "Transmuted state for (bevy_ecs::entity::Entity, bevy_ecs::query::filter::Changed<bevy_ecs::query::state::tests::B>) attempts to access terms that are not allowed by original state (&bevy_ecs::query::state::tests::A, ())."
    )]
    fn cannot_transmute_changed_without_access() {
        let mut world = World::new();
        world.init_component::<A>();
        world.init_component::<B>();
        let query = QueryState::<&A>::new(&mut world);
        let _new_query = query.transmute_filtered::<Entity, Changed<B>>(world.components());
    }

    #[test]
    fn join() {
        let mut world = World::new();
        world.spawn(A(0));
        world.spawn(B(1));
        let entity_ab = world.spawn((A(2), B(3))).id();
        world.spawn((A(4), B(5), C(6)));

        let query_1 = QueryState::<&A, Without<C>>::new(&mut world);
        let query_2 = QueryState::<&B, Without<C>>::new(&mut world);
        let mut new_query: QueryState<Entity, ()> =
            query_1.join_filtered(world.components(), &query_2);

        assert_eq!(new_query.single(&world), entity_ab);
    }

    #[test]
    fn join_with_get() {
        let mut world = World::new();
        world.spawn(A(0));
        world.spawn(B(1));
        let entity_ab = world.spawn((A(2), B(3))).id();
        let entity_abc = world.spawn((A(4), B(5), C(6))).id();

        let query_1 = QueryState::<&A>::new(&mut world);
        let query_2 = QueryState::<&B, Without<C>>::new(&mut world);
        let mut new_query: QueryState<Entity, ()> =
            query_1.join_filtered(world.components(), &query_2);

        assert!(new_query.get(&world, entity_ab).is_ok());
        // should not be able to get entity with c.
        assert!(new_query.get(&world, entity_abc).is_err());
    }

    #[test]
    #[should_panic(expected = "Joined state for (&bevy_ecs::query::state::tests::C, ()) \
            attempts to access terms that are not allowed by state \
            (&bevy_ecs::query::state::tests::A, ()) joined with (&bevy_ecs::query::state::tests::B, ()).")]
    fn cannot_join_wrong_fetch() {
        let mut world = World::new();
        world.init_component::<C>();
        let query_1 = QueryState::<&A>::new(&mut world);
        let query_2 = QueryState::<&B>::new(&mut world);
        let _query: QueryState<&C> = query_1.join(world.components(), &query_2);
    }

    #[test]
    #[should_panic(
        expected = "Joined state for (bevy_ecs::entity::Entity, bevy_ecs::query::filter::Changed<bevy_ecs::query::state::tests::C>) \
            attempts to access terms that are not allowed by state \
            (&bevy_ecs::query::state::tests::A, bevy_ecs::query::filter::Without<bevy_ecs::query::state::tests::C>) \
            joined with (&bevy_ecs::query::state::tests::B, bevy_ecs::query::filter::Without<bevy_ecs::query::state::tests::C>)."
    )]
    fn cannot_join_wrong_filter() {
        let mut world = World::new();
        let query_1 = QueryState::<&A, Without<C>>::new(&mut world);
        let query_2 = QueryState::<&B, Without<C>>::new(&mut world);
        let _: QueryState<Entity, Changed<C>> = query_1.join_filtered(world.components(), &query_2);
    }
}
