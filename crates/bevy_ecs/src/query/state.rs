use crate::{
    archetype::{Archetype, ArchetypeGeneration, ArchetypeId},
    component::{ComponentId, Tick},
    entity::{Entity, EntityEquivalent, EntitySet, UniqueEntityArray},
    entity_disabling::DefaultQueryFilters,
    prelude::FromWorld,
    query::{FilteredAccess, QueryCombinationIter, QueryIter, QueryParIter, WorldQuery},
    storage::{SparseSetIndex, TableId},
    system::Query,
    world::{unsafe_world_cell::UnsafeWorldCell, World, WorldId},
};

#[cfg(all(not(target_arch = "wasm32"), feature = "multi_threaded"))]
use crate::entity::UniqueEntityEquivalentSlice;

use alloc::vec::Vec;
use bevy_utils::prelude::DebugName;
use core::{fmt, ptr};
use fixedbitset::FixedBitSet;
use log::warn;
#[cfg(feature = "trace")]
use tracing::Span;

use super::{
    NopWorldQuery, QueryBuilder, QueryData, QueryEntityError, QueryFilter, QueryManyIter,
    QueryManyUniqueIter, QuerySingleError, ROQueryItem, ReadOnlyQueryData,
};

/// An ID for either a table or an archetype. Used for Query iteration.
///
/// Query iteration is exclusively dense (over tables) or archetypal (over archetypes) based on whether
/// the query filters are dense or not. This is represented by the [`QueryState::is_dense`] field.
///
/// Note that `D::IS_DENSE` and `F::IS_DENSE` have no relationship with `QueryState::is_dense` and
/// any combination of their values can happen.
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
///   that the query will iterate over the data in the matched table/archetype.
/// - cache the [`State`] needed to compute the [`Fetch`] struct used to retrieve data
///   from a specific [`Table`] or [`Archetype`]
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
    /// Note that because we do a zero-cost reference conversion in `Query::as_readonly`,
    /// the access for a read-only query may include accesses for the original mutable version,
    /// but the `Query` does not have exclusive access to those components.
    pub(crate) component_access: FilteredAccess<ComponentId>,
    // NOTE: we maintain both a bitset and a vec because iterating the vec is faster
    pub(super) matched_storage_ids: Vec<StorageId>,
    // Represents whether this query iteration is dense or not. When this is true
    // `matched_storage_ids` stores `TableId`s, otherwise it stores `ArchetypeId`s.
    pub(super) is_dense: bool,
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
    /// # Safety
    ///
    /// `NewD` must have a subset of the access that `D` does and match the exact same archetypes/tables
    /// `NewF` must have a subset of the access that `F` does and match the exact same archetypes/tables
    pub(crate) unsafe fn as_transmuted_state<
        NewD: ReadOnlyQueryData<State = D::State>,
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

    /// Creates a new [`QueryState`] from a given [`World`] and inherits the result of `world.id()`.
    pub fn new(world: &mut World) -> Self {
        let mut state = Self::new_uninitialized(world);
        state.update_archetypes(world);
        state
    }

    /// Creates a new [`QueryState`] from an immutable [`World`] reference and inherits the result of `world.id()`.
    ///
    /// This function may fail if, for example,
    /// the components that make up this query have not been registered into the world.
    pub fn try_new(world: &World) -> Option<Self> {
        let mut state = Self::try_new_uninitialized(world)?;
        state.update_archetypes(world);
        Some(state)
    }

    /// Creates a new [`QueryState`] but does not populate it with the matched results from the World yet
    ///
    /// `new_archetype` and its variants must be called on all of the World's archetypes before the
    /// state can return valid query results.
    fn new_uninitialized(world: &mut World) -> Self {
        let fetch_state = D::init_state(world);
        let filter_state = F::init_state(world);
        Self::from_states_uninitialized(world, fetch_state, filter_state)
    }

    /// Creates a new [`QueryState`] but does not populate it with the matched results from the World yet
    ///
    /// `new_archetype` and its variants must be called on all of the World's archetypes before the
    /// state can return valid query results.
    fn try_new_uninitialized(world: &World) -> Option<Self> {
        let fetch_state = D::get_state(world.components())?;
        let filter_state = F::get_state(world.components())?;
        Some(Self::from_states_uninitialized(
            world,
            fetch_state,
            filter_state,
        ))
    }

    /// Creates a new [`QueryState`] but does not populate it with the matched results from the World yet
    ///
    /// `new_archetype` and its variants must be called on all of the World's archetypes before the
    /// state can return valid query results.
    fn from_states_uninitialized(
        world: &World,
        fetch_state: <D as WorldQuery>::State,
        filter_state: <F as WorldQuery>::State,
    ) -> Self {
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

        // For queries without dynamic filters the dense-ness of the query is equal to the dense-ness
        // of its static type parameters.
        let mut is_dense = D::IS_DENSE && F::IS_DENSE;

        if let Some(default_filters) = world.get_resource::<DefaultQueryFilters>() {
            default_filters.modify_access(&mut component_access);
            is_dense &= default_filters.is_dense(world.components());
        }

        Self {
            world_id: world.id(),
            archetype_generation: ArchetypeGeneration::initial(),
            matched_storage_ids: Vec::new(),
            is_dense,
            fetch_state,
            filter_state,
            component_access,
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            #[cfg(feature = "trace")]
            par_iter_span: tracing::info_span!(
                "par_for_each",
                query = core::any::type_name::<D>(),
                filter = core::any::type_name::<F>(),
            ),
        }
    }

    /// Creates a new [`QueryState`] from a given [`QueryBuilder`] and inherits its [`FilteredAccess`].
    pub fn from_builder(builder: &mut QueryBuilder<D, F>) -> Self {
        let mut fetch_state = D::init_state(builder.world_mut());
        let filter_state = F::init_state(builder.world_mut());

        let mut component_access = FilteredAccess::default();
        D::update_component_access(&fetch_state, &mut component_access);
        D::provide_extra_access(
            &mut fetch_state,
            component_access.access_mut(),
            builder.access().access(),
        );

        let mut component_access = builder.access().clone();

        // For dynamic queries the dense-ness is given by the query builder.
        let mut is_dense = builder.is_dense();

        if let Some(default_filters) = builder.world().get_resource::<DefaultQueryFilters>() {
            default_filters.modify_access(&mut component_access);
            is_dense &= default_filters.is_dense(builder.world().components());
        }

        let mut state = Self {
            world_id: builder.world().id(),
            archetype_generation: ArchetypeGeneration::initial(),
            matched_storage_ids: Vec::new(),
            is_dense,
            fetch_state,
            filter_state,
            component_access,
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            #[cfg(feature = "trace")]
            par_iter_span: tracing::info_span!(
                "par_for_each",
                data = core::any::type_name::<D>(),
                filter = core::any::type_name::<F>(),
            ),
        };
        state.update_archetypes(builder.world());
        state
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// This will create read-only queries, see [`Self::query_mut`] for mutable queries.
    pub fn query<'w, 's>(&'s mut self, world: &'w World) -> Query<'w, 's, D::ReadOnly, F> {
        self.update_archetypes(world);
        self.query_manual(world)
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// This method is slightly more efficient than [`QueryState::query`] in some situations, since
    /// it does not update this instance's internal cache. The resulting query may skip an entity that
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], [`QueryState::get`], or any method with mutable
    /// access to `self`.
    ///
    /// This will create read-only queries, see [`Self::query_mut`] for mutable queries.
    pub fn query_manual<'w, 's>(&'s self, world: &'w World) -> Query<'w, 's, D::ReadOnly, F> {
        self.validate_world(world.id());
        // SAFETY:
        // - We have read access to the entire world, and we call `as_readonly()` so the query only performs read access.
        // - We called `validate_world`.
        unsafe {
            self.as_readonly()
                .query_unchecked_manual(world.as_unsafe_world_cell_readonly())
        }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    pub fn query_mut<'w, 's>(&'s mut self, world: &'w mut World) -> Query<'w, 's, D, F> {
        let last_run = world.last_change_tick();
        let this_run = world.change_tick();
        // SAFETY: We have exclusive access to the entire world.
        unsafe { self.query_unchecked_with_ticks(world.as_unsafe_world_cell(), last_run, this_run) }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub unsafe fn query_unchecked<'w, 's>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
    ) -> Query<'w, 's, D, F> {
        self.update_archetypes_unsafe_world_cell(world);
        // SAFETY: Caller ensures we have the required access
        unsafe { self.query_unchecked_manual(world) }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// This method is slightly more efficient than [`QueryState::query_unchecked`] in some situations, since
    /// it does not update this instance's internal cache. The resulting query may skip an entity that
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], [`QueryState::get`], or any method with mutable
    /// access to `self`.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    pub unsafe fn query_unchecked_manual<'w, 's>(
        &'s self,
        world: UnsafeWorldCell<'w>,
    ) -> Query<'w, 's, D, F> {
        let last_run = world.last_change_tick();
        let this_run = world.change_tick();
        // SAFETY:
        // - The caller ensured we have the correct access to the world.
        // - The caller ensured that the world matches.
        unsafe { self.query_unchecked_manual_with_ticks(world, last_run, this_run) }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    pub unsafe fn query_unchecked_with_ticks<'w, 's>(
        &'s mut self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Query<'w, 's, D, F> {
        self.update_archetypes_unsafe_world_cell(world);
        // SAFETY:
        // - The caller ensured we have the correct access to the world.
        // - We called `update_archetypes_unsafe_world_cell`, which calls `validate_world`.
        unsafe { self.query_unchecked_manual_with_ticks(world, last_run, this_run) }
    }

    /// Creates a [`Query`] from the given [`QueryState`] and [`World`].
    ///
    /// This method is slightly more efficient than [`QueryState::query_unchecked_with_ticks`] in some situations, since
    /// it does not update this instance's internal cache. The resulting query may skip an entity that
    /// belongs to an archetype that has not been cached.
    ///
    /// To ensure that the cache is up to date, call [`QueryState::update_archetypes`] before this method.
    /// The cache is also updated in [`QueryState::new`], [`QueryState::get`], or any method with mutable
    /// access to `self`.
    ///
    /// # Safety
    ///
    /// This does not check for mutable query correctness. To be safe, make sure mutable queries
    /// have unique access to the components they query.
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    pub unsafe fn query_unchecked_manual_with_ticks<'w, 's>(
        &'s self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Query<'w, 's, D, F> {
        // SAFETY:
        // - The caller ensured we have the correct access to the world.
        // - The caller ensured that the world matches.
        unsafe { Query::new(world, self, last_run, this_run) }
    }

    /// Checks if the query is empty for the given [`World`], where the last change and current tick are given.
    ///
    /// This is equivalent to `self.iter().next().is_none()`, and thus the worst case runtime will be `O(n)`
    /// where `n` is the number of *potential* matches. This can be notably expensive for queries that rely
    /// on non-archetypal filters such as [`Added`], [`Changed`] or [`Spawned`] which must individually check
    /// each query result for a match.
    ///
    /// # Panics
    ///
    /// If `world` does not match the one used to call `QueryState::new` for this instance.
    ///
    /// [`Added`]: crate::query::Added
    /// [`Changed`]: crate::query::Changed
    /// [`Spawned`]: crate::query::Spawned
    #[inline]
    pub fn is_empty(&self, world: &World, last_run: Tick, this_run: Tick) -> bool {
        self.validate_world(world.id());
        // SAFETY:
        // - We have read access to the entire world, and `is_empty()` only performs read access.
        // - We called `validate_world`.
        unsafe {
            self.query_unchecked_manual_with_ticks(
                world.as_unsafe_world_cell_readonly(),
                last_run,
                this_run,
            )
        }
        .is_empty()
    }

    /// Returns `true` if the given [`Entity`] matches the query.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn contains(&self, entity: Entity, world: &World, last_run: Tick, this_run: Tick) -> bool {
        self.validate_world(world.id());
        // SAFETY:
        // - We have read access to the entire world, and `is_empty()` only performs read access.
        // - We called `validate_world`.
        unsafe {
            self.query_unchecked_manual_with_ticks(
                world.as_unsafe_world_cell_readonly(),
                last_run,
                this_run,
            )
        }
        .contains(entity)
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
        if self.component_access.required.is_empty() {
            let archetypes = world.archetypes();
            let old_generation =
                core::mem::replace(&mut self.archetype_generation, archetypes.generation());

            for archetype in &archetypes[old_generation..] {
                // SAFETY: The validate_world call ensures that the world is the same the QueryState
                // was initialized from.
                unsafe {
                    self.new_archetype(archetype);
                }
            }
        } else {
            // skip if we are already up to date
            if self.archetype_generation == world.archetypes().generation() {
                return;
            }
            // if there are required components, we can optimize by only iterating through archetypes
            // that contain at least one of the required components
            let potential_archetypes = self
                .component_access
                .required
                .ones()
                .filter_map(|idx| {
                    let component_id = ComponentId::get_sparse_set_index(idx);
                    world
                        .archetypes()
                        .component_index()
                        .get(&component_id)
                        .map(|index| index.keys())
                })
                // select the component with the fewest archetypes
                .min_by_key(ExactSizeIterator::len);
            if let Some(archetypes) = potential_archetypes {
                for archetype_id in archetypes {
                    // exclude archetypes that have already been processed
                    if archetype_id < &self.archetype_generation.0 {
                        continue;
                    }
                    // SAFETY: get_potential_archetypes only returns archetype ids that are valid for the world
                    let archetype = &world.archetypes()[*archetype_id];
                    // SAFETY: The validate_world call ensures that the world is the same the QueryState
                    // was initialized from.
                    unsafe {
                        self.new_archetype(archetype);
                    }
                }
            }
            self.archetype_generation = world.archetypes().generation();
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
    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    pub unsafe fn new_archetype(&mut self, archetype: &Archetype) {
        if D::matches_component_set(&self.fetch_state, &|id| archetype.contains(id))
            && F::matches_component_set(&self.filter_state, &|id| archetype.contains(id))
            && self.matches_component_set(&|id| archetype.contains(id))
        {
            let archetype_index = archetype.id().index();
            if !self.matched_archetypes.contains(archetype_index) {
                self.matched_archetypes.grow_and_insert(archetype_index);
                if !self.is_dense {
                    self.matched_storage_ids.push(StorageId {
                        archetype_id: archetype.id(),
                    });
                }
            }
            let table_index = archetype.table_id().as_usize();
            if !self.matched_tables.contains(table_index) {
                self.matched_tables.grow_and_insert(table_index);
                if self.is_dense {
                    self.matched_storage_ids.push(StorageId {
                        table_id: archetype.table_id(),
                    });
                }
            }
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

    /// Use this to transform a [`QueryState`] into a more generic [`QueryState`].
    /// This can be useful for passing to another function that might take the more general form.
    /// See [`Query::transmute_lens`](crate::system::Query::transmute_lens) for more details.
    ///
    /// You should not call [`update_archetypes`](Self::update_archetypes) on the returned [`QueryState`] as the result will be unpredictable.
    /// You might end up with a mix of archetypes that only matched the original query + archetypes that only match
    /// the new [`QueryState`]. Most of the safe methods on [`QueryState`] call [`QueryState::update_archetypes`] internally, so this
    /// best used through a [`Query`]
    pub fn transmute<'a, NewD: QueryData>(
        &self,
        world: impl Into<UnsafeWorldCell<'a>>,
    ) -> QueryState<NewD> {
        self.transmute_filtered::<NewD, ()>(world.into())
    }

    /// Creates a new [`QueryState`] with the same underlying [`FilteredAccess`], matched tables and archetypes
    /// as self but with a new type signature.
    ///
    /// Panics if `NewD` or `NewF` require accesses that this query does not have.
    pub fn transmute_filtered<'a, NewD: QueryData, NewF: QueryFilter>(
        &self,
        world: impl Into<UnsafeWorldCell<'a>>,
    ) -> QueryState<NewD, NewF> {
        let world = world.into();
        self.validate_world(world.id());

        let mut component_access = FilteredAccess::default();
        let mut fetch_state = NewD::get_state(world.components()).expect("Could not create fetch_state, Please initialize all referenced components before transmuting.");
        let filter_state = NewF::get_state(world.components()).expect("Could not create filter_state, Please initialize all referenced components before transmuting.");

        let mut self_access = self.component_access.clone();
        if D::IS_READ_ONLY {
            // The current state was transmuted from a mutable
            // `QueryData` to a read-only one.
            // Ignore any write access in the current state.
            self_access.access_mut().clear_writes();
        }

        NewD::update_component_access(&fetch_state, &mut component_access);
        NewD::provide_extra_access(
            &mut fetch_state,
            component_access.access_mut(),
            self_access.access(),
        );

        let mut filter_component_access = FilteredAccess::default();
        NewF::update_component_access(&filter_state, &mut filter_component_access);

        component_access.extend(&filter_component_access);
        assert!(
            component_access.is_subset(&self_access),
            "Transmuted state for {} attempts to access terms that are not allowed by original state {}.",
            DebugName::type_name::<(NewD, NewF)>(), DebugName::type_name::<(D, F)>()
        );

        QueryState {
            world_id: self.world_id,
            archetype_generation: self.archetype_generation,
            matched_storage_ids: self.matched_storage_ids.clone(),
            is_dense: self.is_dense,
            fetch_state,
            filter_state,
            component_access: self_access,
            matched_tables: self.matched_tables.clone(),
            matched_archetypes: self.matched_archetypes.clone(),
            #[cfg(feature = "trace")]
            par_iter_span: tracing::info_span!(
                "par_for_each",
                query = core::any::type_name::<NewD>(),
                filter = core::any::type_name::<NewF>(),
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
    pub fn join<'a, OtherD: QueryData, NewD: QueryData>(
        &self,
        world: impl Into<UnsafeWorldCell<'a>>,
        other: &QueryState<OtherD>,
    ) -> QueryState<NewD, ()> {
        self.join_filtered::<_, (), NewD, ()>(world, other)
    }

    /// Use this to combine two queries. The data accessed will be the intersection
    /// of archetypes included in both queries.
    ///
    /// ## Panics
    ///
    /// Will panic if `NewD` or `NewF` requires accesses not in `Q` or `OtherQ`.
    pub fn join_filtered<
        'a,
        OtherD: QueryData,
        OtherF: QueryFilter,
        NewD: QueryData,
        NewF: QueryFilter,
    >(
        &self,
        world: impl Into<UnsafeWorldCell<'a>>,
        other: &QueryState<OtherD, OtherF>,
    ) -> QueryState<NewD, NewF> {
        if self.world_id != other.world_id {
            panic!("Joining queries initialized on different worlds is not allowed.");
        }

        let world = world.into();

        self.validate_world(world.id());

        let mut component_access = FilteredAccess::default();
        let mut new_fetch_state = NewD::get_state(world.components())
            .expect("Could not create fetch_state, Please initialize all referenced components before transmuting.");
        let new_filter_state = NewF::get_state(world.components())
            .expect("Could not create filter_state, Please initialize all referenced components before transmuting.");

        let mut joined_component_access = self.component_access.clone();
        joined_component_access.extend(&other.component_access);

        if D::IS_READ_ONLY && self.component_access.access().has_any_write()
            || OtherD::IS_READ_ONLY && other.component_access.access().has_any_write()
        {
            // One of the input states was transmuted from a mutable
            // `QueryData` to a read-only one.
            // Ignore any write access in that current state.
            // The simplest way to do this is to clear *all* writes
            // and then add back in any writes that are valid
            joined_component_access.access_mut().clear_writes();
            if !D::IS_READ_ONLY {
                joined_component_access
                    .access_mut()
                    .extend(self.component_access.access());
            }
            if !OtherD::IS_READ_ONLY {
                joined_component_access
                    .access_mut()
                    .extend(other.component_access.access());
            }
        }

        NewD::update_component_access(&new_fetch_state, &mut component_access);
        NewD::provide_extra_access(
            &mut new_fetch_state,
            component_access.access_mut(),
            joined_component_access.access(),
        );

        let mut new_filter_component_access = FilteredAccess::default();
        NewF::update_component_access(&new_filter_state, &mut new_filter_component_access);

        component_access.extend(&new_filter_component_access);

        assert!(
            component_access.is_subset(&joined_component_access),
            "Joined state for {} attempts to access terms that are not allowed by state {} joined with {}.",
            DebugName::type_name::<(NewD, NewF)>(), DebugName::type_name::<(D, F)>(), DebugName::type_name::<(OtherD, OtherF)>()
        );

        if self.archetype_generation != other.archetype_generation {
            warn!("You have tried to join queries with different archetype_generations. This could lead to unpredictable results.");
        }

        // the join is dense of both the queries were dense.
        let is_dense = self.is_dense && other.is_dense;

        // take the intersection of the matched ids
        let mut matched_tables = self.matched_tables.clone();
        let mut matched_archetypes = self.matched_archetypes.clone();
        matched_tables.intersect_with(&other.matched_tables);
        matched_archetypes.intersect_with(&other.matched_archetypes);
        let matched_storage_ids = if is_dense {
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
            is_dense,
            fetch_state: new_fetch_state,
            filter_state: new_filter_state,
            component_access: joined_component_access,
            matched_tables,
            matched_archetypes,
            #[cfg(feature = "trace")]
            par_iter_span: tracing::info_span!(
                "par_for_each",
                query = core::any::type_name::<NewD>(),
                filter = core::any::type_name::<NewF>(),
            ),
        }
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This can only be called for read-only queries, see [`Self::get_mut`] for write-queries.
    ///
    /// If you need to get multiple items at once but get borrowing errors,
    /// consider using [`Self::update_archetypes`] followed by multiple [`Self::get_manual`] calls,
    /// or making a single call with [`Self::get_many`]  or [`Self::iter_many`].
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get<'w>(
        &mut self,
        world: &'w World,
        entity: Entity,
    ) -> Result<ROQueryItem<'w, '_, D>, QueryEntityError> {
        self.query(world).get_inner(entity)
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
    /// let wrong_entity = Entity::from_raw_u32(365).unwrap();
    ///
    /// assert_eq!(match query_state.get_many(&mut world, [wrong_entity]).unwrap_err() {QueryEntityError::EntityDoesNotExist(error) => error.entity, _ => panic!()}, wrong_entity);
    /// ```
    #[inline]
    pub fn get_many<'w, const N: usize>(
        &mut self,
        world: &'w World,
        entities: [Entity; N],
    ) -> Result<[ROQueryItem<'w, '_, D>; N], QueryEntityError> {
        self.query(world).get_many_inner(entities)
    }

    /// Returns the read-only query results for the given [`UniqueEntityArray`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_ecs::{prelude::*, query::QueryEntityError, entity::{EntitySetIterator, UniqueEntityArray, UniqueEntityVec}};
    ///
    /// #[derive(Component, PartialEq, Debug)]
    /// struct A(usize);
    ///
    /// let mut world = World::new();
    /// let entity_set: UniqueEntityVec = world.spawn_batch((0..3).map(A)).collect_set();
    /// let entity_set: UniqueEntityArray<3> = entity_set.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    ///
    /// let mut query_state = world.query::<&A>();
    ///
    /// let component_values = query_state.get_many_unique(&world, entity_set).unwrap();
    ///
    /// assert_eq!(component_values, [&A(0), &A(1), &A(2)]);
    ///
    /// let wrong_entity = Entity::from_raw_u32(365).unwrap();
    ///
    /// assert_eq!(match query_state.get_many_unique(&mut world, UniqueEntityArray::from([wrong_entity])).unwrap_err() {QueryEntityError::EntityDoesNotExist(error) => error.entity, _ => panic!()}, wrong_entity);
    /// ```
    #[inline]
    pub fn get_many_unique<'w, const N: usize>(
        &mut self,
        world: &'w World,
        entities: UniqueEntityArray<N>,
    ) -> Result<[ROQueryItem<'w, '_, D>; N], QueryEntityError> {
        self.query(world).get_many_unique_inner(entities)
    }

    /// Gets the query result for the given [`World`] and [`Entity`].
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get_mut<'w>(
        &mut self,
        world: &'w mut World,
        entity: Entity,
    ) -> Result<D::Item<'w, '_>, QueryEntityError> {
        self.query_mut(world).get_inner(entity)
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
    /// let wrong_entity = Entity::from_raw_u32(57).unwrap();
    /// let invalid_entity = world.spawn_empty().id();
    ///
    /// assert_eq!(match query_state.get_many(&mut world, [wrong_entity]).unwrap_err() {QueryEntityError::EntityDoesNotExist(error) => error.entity, _ => panic!()}, wrong_entity);
    /// assert_eq!(match query_state.get_many_mut(&mut world, [invalid_entity]).unwrap_err() {QueryEntityError::QueryDoesNotMatch(entity, _) => entity, _ => panic!()}, invalid_entity);
    /// assert_eq!(query_state.get_many_mut(&mut world, [entities[0], entities[0]]).unwrap_err(), QueryEntityError::AliasedMutability(entities[0]));
    /// ```
    #[inline]
    pub fn get_many_mut<'w, const N: usize>(
        &mut self,
        world: &'w mut World,
        entities: [Entity; N],
    ) -> Result<[D::Item<'w, '_>; N], QueryEntityError> {
        self.query_mut(world).get_many_mut_inner(entities)
    }

    /// Returns the query results for the given [`UniqueEntityArray`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is
    /// returned instead.
    ///
    /// ```
    /// use bevy_ecs::{prelude::*, query::QueryEntityError, entity::{EntitySetIterator, UniqueEntityArray, UniqueEntityVec}};
    ///
    /// #[derive(Component, PartialEq, Debug)]
    /// struct A(usize);
    ///
    /// let mut world = World::new();
    ///
    /// let entity_set: UniqueEntityVec = world.spawn_batch((0..3).map(A)).collect_set();
    /// let entity_set: UniqueEntityArray<3> = entity_set.try_into().unwrap();
    ///
    /// world.spawn(A(73));
    ///
    /// let mut query_state = world.query::<&mut A>();
    ///
    /// let mut mutable_component_values = query_state.get_many_unique_mut(&mut world, entity_set).unwrap();
    ///
    /// for mut a in &mut mutable_component_values {
    ///     a.0 += 5;
    /// }
    ///
    /// let component_values = query_state.get_many_unique(&world, entity_set).unwrap();
    ///
    /// assert_eq!(component_values, [&A(5), &A(6), &A(7)]);
    ///
    /// let wrong_entity = Entity::from_raw_u32(57).unwrap();
    /// let invalid_entity = world.spawn_empty().id();
    ///
    /// assert_eq!(match query_state.get_many_unique(&mut world, UniqueEntityArray::from([wrong_entity])).unwrap_err() {QueryEntityError::EntityDoesNotExist(error) => error.entity, _ => panic!()}, wrong_entity);
    /// assert_eq!(match query_state.get_many_unique_mut(&mut world, UniqueEntityArray::from([invalid_entity])).unwrap_err() {QueryEntityError::QueryDoesNotMatch(entity, _) => entity, _ => panic!()}, invalid_entity);
    /// ```
    #[inline]
    pub fn get_many_unique_mut<'w, const N: usize>(
        &mut self,
        world: &'w mut World,
        entities: UniqueEntityArray<N>,
    ) -> Result<[D::Item<'w, '_>; N], QueryEntityError> {
        self.query_mut(world).get_many_unique_inner(entities)
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
    ) -> Result<ROQueryItem<'w, '_, D>, QueryEntityError> {
        self.query_manual(world).get_inner(entity)
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
    ) -> Result<D::Item<'w, '_>, QueryEntityError> {
        self.query_unchecked(world).get_inner(entity)
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// This can only be called for read-only queries, see [`Self::iter_mut`] for write-queries.
    ///
    /// If you need to iterate multiple times at once but get borrowing errors,
    /// consider using [`Self::update_archetypes`] followed by multiple [`Self::iter_manual`] calls.
    #[inline]
    pub fn iter<'w, 's>(&'s mut self, world: &'w World) -> QueryIter<'w, 's, D::ReadOnly, F> {
        self.query(world).into_iter()
    }

    /// Returns an [`Iterator`] over the query results for the given [`World`].
    ///
    /// This iterator is always guaranteed to return results from each matching entity once and only once.
    /// Iteration order is not guaranteed.
    #[inline]
    pub fn iter_mut<'w, 's>(&'s mut self, world: &'w mut World) -> QueryIter<'w, 's, D, F> {
        self.query_mut(world).into_iter()
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
        self.query_manual(world).into_iter()
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
        self.query(world).iter_combinations_inner()
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
        self.query_mut(world).iter_combinations_inner()
    }

    /// Returns an [`Iterator`] over the read-only query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    ///
    /// If you need to iterate multiple times at once but get borrowing errors,
    /// consider using [`Self::update_archetypes`] followed by multiple [`Self::iter_many_manual`] calls.
    ///
    /// # See also
    ///
    /// - [`iter_many_mut`](Self::iter_many_mut) to get mutable query items.
    #[inline]
    pub fn iter_many<'w, 's, EntityList: IntoIterator<Item: EntityEquivalent>>(
        &'s mut self,
        world: &'w World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, D::ReadOnly, F, EntityList::IntoIter> {
        self.query(world).iter_many_inner(entities)
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
    pub fn iter_many_manual<'w, 's, EntityList: IntoIterator<Item: EntityEquivalent>>(
        &'s self,
        world: &'w World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, D::ReadOnly, F, EntityList::IntoIter> {
        self.query_manual(world).iter_many_inner(entities)
    }

    /// Returns an iterator over the query items generated from an [`Entity`] list.
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    #[inline]
    pub fn iter_many_mut<'w, 's, EntityList: IntoIterator<Item: EntityEquivalent>>(
        &'s mut self,
        world: &'w mut World,
        entities: EntityList,
    ) -> QueryManyIter<'w, 's, D, F, EntityList::IntoIter> {
        self.query_mut(world).iter_many_inner(entities)
    }

    /// Returns an [`Iterator`] over the unique read-only query items generated from an [`EntitySet`].
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    ///
    /// # See also
    ///
    /// - [`iter_many_unique_mut`](Self::iter_many_unique_mut) to get mutable query items.
    #[inline]
    pub fn iter_many_unique<'w, 's, EntityList: EntitySet>(
        &'s mut self,
        world: &'w World,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'w, 's, D::ReadOnly, F, EntityList::IntoIter> {
        self.query(world).iter_many_unique_inner(entities)
    }

    /// Returns an [`Iterator`] over the unique read-only query items generated from an [`EntitySet`].
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
    /// - [`iter_many_unique`](Self::iter_many) to update archetypes.
    /// - [`iter_many`](Self::iter_many) to iterate over a non-unique entity list.
    /// - [`iter_manual`](Self::iter_manual) to iterate over all query items.
    #[inline]
    pub fn iter_many_unique_manual<'w, 's, EntityList: EntitySet>(
        &'s self,
        world: &'w World,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'w, 's, D::ReadOnly, F, EntityList::IntoIter> {
        self.query_manual(world).iter_many_unique_inner(entities)
    }

    /// Returns an iterator over the unique query items generated from an [`EntitySet`].
    ///
    /// Items are returned in the order of the list of entities.
    /// Entities that don't match the query are skipped.
    #[inline]
    pub fn iter_many_unique_mut<'w, 's, EntityList: EntitySet>(
        &'s mut self,
        world: &'w mut World,
        entities: EntityList,
    ) -> QueryManyUniqueIter<'w, 's, D, F, EntityList::IntoIter> {
        self.query_mut(world).iter_many_unique_inner(entities)
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
        self.query_unchecked(world).into_iter()
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
        self.query_unchecked(world).iter_combinations_inner()
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
        self.query(world).par_iter_inner()
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
    /// # let wrong_entity = Entity::from_raw_u32(57).unwrap();
    /// # let invalid_entity = world.spawn_empty().id();
    ///
    /// # assert_eq!(match query_state.get_many(&mut world, [wrong_entity]).unwrap_err() {QueryEntityError::EntityDoesNotExist(error) => error.entity, _ => panic!()}, wrong_entity);
    /// assert_eq!(match query_state.get_many_mut(&mut world, [invalid_entity]).unwrap_err() {QueryEntityError::QueryDoesNotMatch(entity, _) => entity, _ => panic!()}, invalid_entity);
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
        self.query_mut(world).par_iter_inner()
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
    pub(crate) unsafe fn par_fold_init_unchecked_manual<'w, 's, T, FN, INIT>(
        &'s self,
        init_accum: INIT,
        world: UnsafeWorldCell<'w>,
        batch_size: u32,
        func: FN,
        last_run: Tick,
        this_run: Tick,
    ) where
        FN: Fn(T, D::Item<'w, 's>) -> T + Send + Sync + Clone,
        INIT: Fn() -> T + Sync + Send + Clone,
    {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter,QueryState::par_fold_init_unchecked_manual,
        // QueryState::par_many_fold_init_unchecked_manual, QueryState::par_many_unique_fold_init_unchecked_manual
        use arrayvec::ArrayVec;

        bevy_tasks::ComputeTaskPool::get().scope(|scope| {
            // SAFETY: We only access table data that has been registered in `self.component_access`.
            let tables = unsafe { &world.storages().tables };
            let archetypes = world.archetypes();
            let mut batch_queue = ArrayVec::new();
            let mut queue_entity_count = 0;

            // submit a list of storages which smaller than batch_size as single task
            let submit_batch_queue = |queue: &mut ArrayVec<StorageId, 128>| {
                if queue.is_empty() {
                    return;
                }
                let queue = core::mem::take(queue);
                let mut func = func.clone();
                let init_accum = init_accum.clone();
                scope.spawn(async move {
                    #[cfg(feature = "trace")]
                    let _span = self.par_iter_span.enter();
                    let mut iter = self
                        .query_unchecked_manual_with_ticks(world, last_run, this_run)
                        .into_iter();
                    let mut accum = init_accum();
                    for storage_id in queue {
                        accum = iter.fold_over_storage_range(accum, &mut func, storage_id, None);
                    }
                });
            };

            // submit single storage larger than batch_size
            let submit_single = |count, storage_id: StorageId| {
                for offset in (0..count).step_by(batch_size as usize) {
                    let mut func = func.clone();
                    let init_accum = init_accum.clone();
                    let len = batch_size.min(count - offset);
                    let batch = offset..offset + len;
                    scope.spawn(async move {
                        #[cfg(feature = "trace")]
                        let _span = self.par_iter_span.enter();
                        let accum = init_accum();
                        self.query_unchecked_manual_with_ticks(world, last_run, this_run)
                            .into_iter()
                            .fold_over_storage_range(accum, &mut func, storage_id, Some(batch));
                    });
                }
            };

            let storage_entity_count = |storage_id: StorageId| -> u32 {
                if self.is_dense {
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

    /// Runs `func` on each query result in parallel for the given [`EntitySet`],
    /// where the last change and the current change tick are given. This is faster than the
    /// equivalent `iter_many_unique()` method, but cannot be chained like a normal [`Iterator`].
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
    pub(crate) unsafe fn par_many_unique_fold_init_unchecked_manual<'w, 's, T, FN, INIT, E>(
        &'s self,
        init_accum: INIT,
        world: UnsafeWorldCell<'w>,
        entity_list: &UniqueEntityEquivalentSlice<E>,
        batch_size: u32,
        mut func: FN,
        last_run: Tick,
        this_run: Tick,
    ) where
        FN: Fn(T, D::Item<'w, 's>) -> T + Send + Sync + Clone,
        INIT: Fn() -> T + Sync + Send + Clone,
        E: EntityEquivalent + Sync,
    {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter,QueryState::par_fold_init_unchecked_manual
        // QueryState::par_many_fold_init_unchecked_manual, QueryState::par_many_unique_fold_init_unchecked_manual

        bevy_tasks::ComputeTaskPool::get().scope(|scope| {
            let chunks = entity_list.chunks_exact(batch_size as usize);
            let remainder = chunks.remainder();

            for batch in chunks {
                let mut func = func.clone();
                let init_accum = init_accum.clone();
                scope.spawn(async move {
                    #[cfg(feature = "trace")]
                    let _span = self.par_iter_span.enter();
                    let accum = init_accum();
                    self.query_unchecked_manual_with_ticks(world, last_run, this_run)
                        .iter_many_unique_inner(batch)
                        .fold(accum, &mut func);
                });
            }

            #[cfg(feature = "trace")]
            let _span = self.par_iter_span.enter();
            let accum = init_accum();
            self.query_unchecked_manual_with_ticks(world, last_run, this_run)
                .iter_many_unique_inner(remainder)
                .fold(accum, &mut func);
        });
    }
}

impl<D: ReadOnlyQueryData, F: QueryFilter> QueryState<D, F> {
    /// Runs `func` on each read-only query result in parallel for the given [`Entity`] list,
    /// where the last change and the current change tick are given. This is faster than the equivalent
    /// `iter_many()` method, but cannot be chained like a normal [`Iterator`].
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
    pub(crate) unsafe fn par_many_fold_init_unchecked_manual<'w, 's, T, FN, INIT, E>(
        &'s self,
        init_accum: INIT,
        world: UnsafeWorldCell<'w>,
        entity_list: &[E],
        batch_size: u32,
        mut func: FN,
        last_run: Tick,
        this_run: Tick,
    ) where
        FN: Fn(T, D::Item<'w, 's>) -> T + Send + Sync + Clone,
        INIT: Fn() -> T + Sync + Send + Clone,
        E: EntityEquivalent + Sync,
    {
        // NOTE: If you are changing query iteration code, remember to update the following places, where relevant:
        // QueryIter, QueryIterationCursor, QueryManyIter, QueryCombinationIter, QueryState::par_fold_init_unchecked_manual
        // QueryState::par_many_fold_init_unchecked_manual, QueryState::par_many_unique_fold_init_unchecked_manual

        bevy_tasks::ComputeTaskPool::get().scope(|scope| {
            let chunks = entity_list.chunks_exact(batch_size as usize);
            let remainder = chunks.remainder();

            for batch in chunks {
                let mut func = func.clone();
                let init_accum = init_accum.clone();
                scope.spawn(async move {
                    #[cfg(feature = "trace")]
                    let _span = self.par_iter_span.enter();
                    let accum = init_accum();
                    self.query_unchecked_manual_with_ticks(world, last_run, this_run)
                        .iter_many_inner(batch)
                        .fold(accum, &mut func);
                });
            }

            #[cfg(feature = "trace")]
            let _span = self.par_iter_span.enter();
            let accum = init_accum();
            self.query_unchecked_manual_with_ticks(world, last_run, this_run)
                .iter_many_inner(remainder)
                .fold(accum, &mut func);
        });
    }
}

impl<D: QueryData, F: QueryFilter> QueryState<D, F> {
    /// Returns a single immutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// This can only be called for read-only queries,
    /// see [`single_mut`](Self::single_mut) for write-queries.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Example
    ///
    /// Sometimes, you might want to handle the error in a specific way,
    /// generally by spawning the missing entity.
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    /// use bevy_ecs::query::QuerySingleError;
    ///
    /// #[derive(Component)]
    /// struct A(usize);
    ///
    /// fn my_system(query: Query<&A>, mut commands: Commands) {
    ///     match query.single() {
    ///         Ok(a) => (), // Do something with `a`
    ///         Err(err) => match err {
    ///             QuerySingleError::NoEntities(_) => {
    ///                 commands.spawn(A(0));
    ///             }
    ///             QuerySingleError::MultipleEntities(_) => panic!("Multiple entities found!"),
    ///         },
    ///     }
    /// }
    /// ```
    ///
    /// However in most cases, this error can simply be handled with a graceful early return.
    /// If this is an expected failure mode, you can do this using the `let else` pattern like so:
    /// ```rust
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct A(usize);
    ///
    /// fn my_system(query: Query<&A>) {
    ///   let Ok(a) = query.single() else {
    ///     return;
    ///   };
    ///
    ///   // Do something with `a`
    /// }
    /// ```
    ///
    /// If this is unexpected though, you should probably use the `?` operator
    /// in combination with Bevy's error handling apparatus.
    ///
    /// ```rust
    /// use bevy_ecs::prelude::*;
    ///
    /// #[derive(Component)]
    /// struct A(usize);
    ///
    /// fn my_system(query: Query<&A>) -> Result {
    ///  let a = query.single()?;
    ///
    ///  // Do something with `a`
    ///  Ok(())
    /// }
    /// ```
    ///
    /// This allows you to globally control how errors are handled in your application,
    /// by setting up a custom error handler.
    /// See the [`bevy_ecs::error`] module docs for more information!
    /// Commonly, you might want to panic on an error during development, but log the error and continue
    /// execution in production.
    ///
    /// Simply unwrapping the [`Result`] also works, but should generally be reserved for tests.
    #[inline]
    pub fn single<'w>(
        &mut self,
        world: &'w World,
    ) -> Result<ROQueryItem<'w, '_, D>, QuerySingleError> {
        self.query(world).single_inner()
    }

    /// Returns a single mutable query result when there is exactly one entity matching
    /// the query.
    ///
    /// If the number of query results is not exactly one, a [`QuerySingleError`] is returned
    /// instead.
    ///
    /// # Examples
    ///
    /// Please see [`Query::single`] for advice on handling the error.
    #[inline]
    pub fn single_mut<'w>(
        &mut self,
        world: &'w mut World,
    ) -> Result<D::Item<'w, '_>, QuerySingleError> {
        self.query_mut(world).single_inner()
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
    pub unsafe fn single_unchecked<'w>(
        &mut self,
        world: UnsafeWorldCell<'w>,
    ) -> Result<D::Item<'w, '_>, QuerySingleError> {
        self.query_unchecked(world).single_inner()
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
    /// This does not validate that `world.id()` matches `self.world_id`. Calling this on a `world`
    /// with a mismatched [`WorldId`] is unsound.
    #[inline]
    pub unsafe fn single_unchecked_manual<'w>(
        &self,
        world: UnsafeWorldCell<'w>,
        last_run: Tick,
        this_run: Tick,
    ) -> Result<D::Item<'w, '_>, QuerySingleError> {
        // SAFETY:
        // - The caller ensured we have the correct access to the world.
        // - The caller ensured that the world matches.
        self.query_unchecked_manual_with_ticks(world, last_run, this_run)
            .single_inner()
    }
}

impl<D: QueryData, F: QueryFilter> From<QueryBuilder<'_, D, F>> for QueryState<D, F> {
    fn from(mut value: QueryBuilder<D, F>) -> Self {
        QueryState::from_builder(&mut value)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        component::Component,
        entity_disabling::DefaultQueryFilters,
        prelude::*,
        system::{QueryLens, RunSystemOnce},
        world::{FilteredEntityMut, FilteredEntityRef},
    };

    #[test]
    #[should_panic]
    fn right_world_get() {
        let mut world_1 = World::new();
        let world_2 = World::new();

        let mut query_state = world_1.query::<Entity>();
        let _panics = query_state.get(&world_2, Entity::from_raw_u32(0).unwrap());
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
        let mut new_query_state = query_state.transmute::<&A>(&world);
        assert_eq!(new_query_state.iter(&world).len(), 1);
        let a = new_query_state.single(&world).unwrap();

        assert_eq!(a.0, 1);
    }

    #[test]
    fn cannot_get_data_not_in_original_query() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));
        world.spawn((A(1), B(0), C(0)));

        let query_state = world.query_filtered::<(&A, &B), Without<C>>();
        let mut new_query_state = query_state.transmute::<&A>(&world);
        // even though we change the query to not have Without<C>, we do not get the component with C.
        let a = new_query_state.single(&world).unwrap();

        assert_eq!(a.0, 0);
    }

    #[test]
    fn can_transmute_empty_tuple() {
        let mut world = World::new();
        world.register_component::<A>();
        let entity = world.spawn(A(10)).id();

        let q = world.query::<()>();
        let mut q = q.transmute::<Entity>(&world);
        assert_eq!(q.single(&world).unwrap(), entity);
    }

    #[test]
    fn can_transmute_immut_fetch() {
        let mut world = World::new();
        world.spawn(A(10));

        let q = world.query::<&A>();
        let mut new_q = q.transmute::<Ref<A>>(&world);
        assert!(new_q.single(&world).unwrap().is_added());

        let q = world.query::<Ref<A>>();
        let _ = q.transmute::<&A>(&world);
    }

    #[test]
    fn can_transmute_mut_fetch() {
        let mut world = World::new();
        world.spawn(A(0));

        let q = world.query::<&mut A>();
        let _ = q.transmute::<Ref<A>>(&world);
        let _ = q.transmute::<&A>(&world);
    }

    #[test]
    fn can_transmute_entity_mut() {
        let mut world = World::new();
        world.spawn(A(0));

        let q: QueryState<EntityMut<'_>> = world.query::<EntityMut>();
        let _ = q.transmute::<EntityRef>(&world);
    }

    #[test]
    fn can_generalize_with_option() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));

        let query_state = world.query::<(Option<&A>, &B)>();
        let _ = query_state.transmute::<Option<&A>>(&world);
        let _ = query_state.transmute::<&B>(&world);
    }

    #[test]
    #[should_panic]
    fn cannot_transmute_to_include_data_not_in_original_query() {
        let mut world = World::new();
        world.register_component::<A>();
        world.register_component::<B>();
        world.spawn(A(0));

        let query_state = world.query::<&A>();
        let mut _new_query_state = query_state.transmute::<(&A, &B)>(&world);
    }

    #[test]
    #[should_panic]
    fn cannot_transmute_immut_to_mut() {
        let mut world = World::new();
        world.spawn(A(0));

        let query_state = world.query::<&A>();
        let mut _new_query_state = query_state.transmute::<&mut A>(&world);
    }

    #[test]
    #[should_panic]
    fn cannot_transmute_option_to_immut() {
        let mut world = World::new();
        world.spawn(C(0));

        let query_state = world.query::<Option<&A>>();
        let mut new_query_state = query_state.transmute::<&A>(&world);
        let x = new_query_state.single(&world).unwrap();
        assert_eq!(x.0, 1234);
    }

    #[test]
    #[should_panic]
    fn cannot_transmute_entity_ref() {
        let mut world = World::new();
        world.register_component::<A>();

        let q = world.query::<EntityRef>();
        let _ = q.transmute::<&A>(&world);
    }

    #[test]
    fn can_transmute_filtered_entity() {
        let mut world = World::new();
        let entity = world.spawn((A(0), B(1))).id();
        let query = QueryState::<(Entity, &A, &B)>::new(&mut world)
            .transmute::<(Entity, FilteredEntityRef)>(&world);

        let mut query = query;
        // Our result is completely untyped
        let (_entity, entity_ref) = query.single(&world).unwrap();

        assert_eq!(entity, entity_ref.id());
        assert_eq!(0, entity_ref.get::<A>().unwrap().0);
        assert_eq!(1, entity_ref.get::<B>().unwrap().0);
    }

    #[test]
    fn can_transmute_added() {
        let mut world = World::new();
        let entity_a = world.spawn(A(0)).id();

        let mut query = QueryState::<(Entity, &A, Has<B>)>::new(&mut world)
            .transmute_filtered::<(Entity, Has<B>), Added<A>>(&world);

        assert_eq!((entity_a, false), query.single(&world).unwrap());

        world.clear_trackers();

        let entity_b = world.spawn((A(0), B(0))).id();
        assert_eq!((entity_b, true), query.single(&world).unwrap());

        world.clear_trackers();

        assert!(query.single(&world).is_err());
    }

    #[test]
    fn can_transmute_changed() {
        let mut world = World::new();
        let entity_a = world.spawn(A(0)).id();

        let mut detection_query = QueryState::<(Entity, &A)>::new(&mut world)
            .transmute_filtered::<Entity, Changed<A>>(&world);

        let mut change_query = QueryState::<&mut A>::new(&mut world);
        assert_eq!(entity_a, detection_query.single(&world).unwrap());

        world.clear_trackers();

        assert!(detection_query.single(&world).is_err());

        change_query.single_mut(&mut world).unwrap().0 = 1;

        assert_eq!(entity_a, detection_query.single(&world).unwrap());
    }

    #[test]
    #[should_panic]
    fn cannot_transmute_changed_without_access() {
        let mut world = World::new();
        world.register_component::<A>();
        world.register_component::<B>();
        let query = QueryState::<&A>::new(&mut world);
        let _new_query = query.transmute_filtered::<Entity, Changed<B>>(&world);
    }

    #[test]
    #[should_panic]
    fn cannot_transmute_mutable_after_readonly() {
        let mut world = World::new();
        // Calling this method would mean we had aliasing queries.
        fn bad(_: Query<&mut A>, _: Query<&A>) {}
        world
            .run_system_once(|query: Query<&mut A>| {
                let mut readonly = query.as_readonly();
                let mut lens: QueryLens<&mut A> = readonly.transmute_lens();
                bad(lens.query(), query.as_readonly());
            })
            .unwrap();
    }

    // Regression test for #14629
    #[test]
    #[should_panic]
    fn transmute_with_different_world() {
        let mut world = World::new();
        world.spawn((A(1), B(2)));

        let mut world2 = World::new();
        world2.register_component::<B>();

        world.query::<(&A, &B)>().transmute::<&B>(&world2);
    }

    /// Regression test for issue #14528
    #[test]
    fn transmute_from_sparse_to_dense() {
        #[derive(Component)]
        struct Dense;

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();

        world.spawn(Dense);
        world.spawn((Dense, Sparse));

        let mut query = world
            .query_filtered::<&Dense, With<Sparse>>()
            .transmute::<&Dense>(&world);

        let matched = query.iter(&world).count();
        assert_eq!(matched, 1);
    }
    #[test]
    fn transmute_from_dense_to_sparse() {
        #[derive(Component)]
        struct Dense;

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();

        world.spawn(Dense);
        world.spawn((Dense, Sparse));

        let mut query = world
            .query::<&Dense>()
            .transmute_filtered::<&Dense, With<Sparse>>(&world);

        // Note: `transmute_filtered` is supposed to keep the same matched tables/archetypes,
        // so it doesn't actually filter out those entities without `Sparse` and the iteration
        // remains dense.
        let matched = query.iter(&world).count();
        assert_eq!(matched, 2);
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
        let mut new_query: QueryState<Entity, ()> = query_1.join_filtered(&world, &query_2);

        assert_eq!(new_query.single(&world).unwrap(), entity_ab);
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
        let mut new_query: QueryState<Entity, ()> = query_1.join_filtered(&world, &query_2);

        assert!(new_query.get(&world, entity_ab).is_ok());
        // should not be able to get entity with c.
        assert!(new_query.get(&world, entity_abc).is_err());
    }

    #[test]
    #[should_panic]
    fn cannot_join_wrong_fetch() {
        let mut world = World::new();
        world.register_component::<C>();
        let query_1 = QueryState::<&A>::new(&mut world);
        let query_2 = QueryState::<&B>::new(&mut world);
        let _query: QueryState<&C> = query_1.join(&world, &query_2);
    }

    #[test]
    #[should_panic]
    fn cannot_join_wrong_filter() {
        let mut world = World::new();
        let query_1 = QueryState::<&A, Without<C>>::new(&mut world);
        let query_2 = QueryState::<&B, Without<C>>::new(&mut world);
        let _: QueryState<Entity, Changed<C>> = query_1.join_filtered(&world, &query_2);
    }

    #[test]
    #[should_panic]
    fn cannot_join_mutable_after_readonly() {
        let mut world = World::new();
        // Calling this method would mean we had aliasing queries.
        fn bad(_: Query<(&mut A, &mut B)>, _: Query<&A>) {}
        world
            .run_system_once(|query_a: Query<&mut A>, mut query_b: Query<&mut B>| {
                let mut readonly = query_a.as_readonly();
                let mut lens: QueryLens<(&mut A, &mut B)> = readonly.join(&mut query_b);
                bad(lens.query(), query_a.as_readonly());
            })
            .unwrap();
    }

    #[test]
    fn join_to_filtered_entity_mut() {
        let mut world = World::new();
        world.spawn((A(2), B(3)));

        let query_1 = QueryState::<&mut A>::new(&mut world);
        let query_2 = QueryState::<&mut B>::new(&mut world);
        let mut new_query: QueryState<(Entity, FilteredEntityMut)> = query_1.join(&world, &query_2);

        let (_entity, mut entity_mut) = new_query.single_mut(&mut world).unwrap();
        assert!(entity_mut.get_mut::<A>().is_some());
        assert!(entity_mut.get_mut::<B>().is_some());
    }

    #[test]
    fn query_respects_default_filters() {
        let mut world = World::new();
        world.spawn((A(0), B(0)));
        world.spawn((B(0), C(0)));
        world.spawn(C(0));

        let mut df = DefaultQueryFilters::empty();
        df.register_disabling_component(world.register_component::<C>());
        world.insert_resource(df);

        // Without<C> only matches the first entity
        let mut query = QueryState::<()>::new(&mut world);
        assert_eq!(1, query.iter(&world).count());

        // With<C> matches the last two entities
        let mut query = QueryState::<(), With<C>>::new(&mut world);
        assert_eq!(2, query.iter(&world).count());

        // Has should bypass the filter entirely
        let mut query = QueryState::<Has<C>>::new(&mut world);
        assert_eq!(3, query.iter(&world).count());

        // Allows should bypass the filter entirely
        let mut query = QueryState::<(), Allows<C>>::new(&mut world);
        assert_eq!(3, query.iter(&world).count());

        // Other filters should still be respected
        let mut query = QueryState::<Has<C>, Without<B>>::new(&mut world);
        assert_eq!(1, query.iter(&world).count());
    }

    #[derive(Component)]
    struct Table;

    #[derive(Component)]
    #[component(storage = "SparseSet")]
    struct Sparse;

    #[test]
    fn query_default_filters_updates_is_dense() {
        let mut world = World::new();
        world.spawn((Table, Sparse));
        world.spawn(Table);
        world.spawn(Sparse);

        let mut query = QueryState::<()>::new(&mut world);
        // There are no sparse components involved thus the query is dense
        assert!(query.is_dense);
        assert_eq!(3, query.iter(&world).count());

        let mut df = DefaultQueryFilters::empty();
        df.register_disabling_component(world.register_component::<Sparse>());
        world.insert_resource(df);

        let mut query = QueryState::<()>::new(&mut world);
        // The query doesn't ask for sparse components, but the default filters adds
        // a sparse components thus it is NOT dense
        assert!(!query.is_dense);
        assert_eq!(1, query.iter(&world).count());

        let mut df = DefaultQueryFilters::empty();
        df.register_disabling_component(world.register_component::<Table>());
        world.insert_resource(df);

        let mut query = QueryState::<()>::new(&mut world);
        // If the filter is instead a table components, the query can still be dense
        assert!(query.is_dense);
        assert_eq!(1, query.iter(&world).count());

        let mut query = QueryState::<&Sparse>::new(&mut world);
        // But only if the original query was dense
        assert!(!query.is_dense);
        assert_eq!(1, query.iter(&world).count());
    }
}
