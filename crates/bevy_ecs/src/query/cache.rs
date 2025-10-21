use crate::query::QueryState;
use alloc::borrow::Cow;
use alloc::vec::Vec;
use bevy_ecs::archetype::{Archetype, ArchetypeGeneration, ArchetypeId, Archetypes};
use bevy_ecs::component::ComponentId;
use bevy_ecs::prelude::World;
use bevy_ecs::query::state::StorageId;
use bevy_ecs::query::{FilteredAccess, QueryBuilder, QueryData, QueryFilter};
use bevy_ecs::storage::{SparseSetIndex, TableId};
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use bevy_ecs::world::WorldId;
use core::fmt::{Debug, Formatter};
use fixedbitset::FixedBitSet;
use log::warn;

/// Borrow-only view over the non-cache fields of a QueryState.
#[doc(hidden)]
pub struct UncachedQueryBorrow<'a, D: QueryData, F: QueryFilter> {
    pub(crate) world_id: WorldId,
    pub(crate) component_access: &'a FilteredAccess,
    pub(crate) fetch_state: &'a D::State,
    pub(crate) filter_state: &'a F::State,
}

impl<'a, D: QueryData, F: QueryFilter> UncachedQueryBorrow<'a, D, F> {
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

    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    ///
    /// # Safety
    /// `archetype` must be from the `World` this [`UncachedQueryBorrow`] was initialized from.
    pub unsafe fn matches_archetype(&self, archetype: &Archetype) -> bool {
        D::matches_component_set(self.fetch_state, &|id| archetype.contains(id))
            && F::matches_component_set(self.filter_state, &|id| archetype.contains(id))
            && self.component_access.filter_sets.iter().any(|set| {
                set.with
                    .ones()
                    .all(|index| archetype.contains(ComponentId::get_sparse_set_index(index)))
                    && set
                        .without
                        .ones()
                        .all(|index| !archetype.contains(ComponentId::get_sparse_set_index(index)))
            })
    }

    /// Iterate through all new archetypes more recent than the provided [`ArchetypeGeneration`],
    /// and call `f` on each of them.
    pub fn iter_archetypes(
        &self,
        archetype_generation: ArchetypeGeneration,
        archetypes: &Archetypes,
        mut f: impl FnMut(&Archetype),
    ) {
        if self.component_access.required.is_empty() {
            archetypes[archetype_generation..]
                .iter()
                .for_each(|archetype| {
                    // SAFETY: The validate_world call ensures that the world is the same the QueryState
                    // was initialized from.
                    if unsafe { self.matches_archetype(archetype) } {
                        f(archetype);
                    }
                });
        } else {
            // if there are required components, we can optimize by only iterating through archetypes
            // that contain at least one of the required components
            let potential_archetype_ids = self
                .component_access
                .required
                .ones()
                .filter_map(|idx| {
                    let component_id = ComponentId::get_sparse_set_index(idx);
                    archetypes
                        .component_index()
                        .get(&component_id)
                        .map(|index| index.keys())
                })
                // select the component with the fewest archetypes
                .min_by_key(ExactSizeIterator::len);
            if let Some(archetype_ids) = potential_archetype_ids {
                for archetype_id in archetype_ids {
                    // exclude archetypes that have already been processed
                    if archetype_id < &archetype_generation.0 {
                        continue;
                    }
                    // SAFETY: get_potential_archetypes only returns archetype ids that are valid for the world
                    let archetype = &archetypes[*archetype_id];
                    // SAFETY: The validate_world call ensures that the world is the same the QueryState
                    // was initialized from.
                    if unsafe { self.matches_archetype(archetype) } {
                        f(archetype);
                    }
                }
            }
        }
    }
}

impl<D: QueryData, F: QueryFilter, C: QueryCache> QueryState<D, F, C> {
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
        self.split_cache(|prefix, cache| {
            cache.update_archetypes(&prefix, world);
        });
    }

    /// Returns `true` if this query matches this [`Archetype`]. Otherwise, returns `false`.
    ///
    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    pub(crate) unsafe fn matches_archetype(&self, archetype: &Archetype) -> bool {
        // SAFETY: from parent function's safety constraint
        unsafe { self.cache.contains(self, archetype) }
    }
}

impl<D: QueryData, F: QueryFilter> QueryState<D, F, Uncached> {
    /// Iterate through all archetypes that match the [`QueryState`] with an [`ArchetypeGeneration`] higher than the provided one,
    /// and call `f` on each of them.
    fn iter_archetypes(
        &self,
        archetype_generation: ArchetypeGeneration,
        archetypes: &Archetypes,
        f: impl FnMut(&Archetype),
    ) {
        self.as_uncached()
            .iter_archetypes(archetype_generation, archetypes, f);
    }
}

/// Types that can cache archetypes matched by a `Query`.
pub trait QueryCache: Debug + Clone + Sync {
    /// Returns the data needed to iterate through the archetypes that match the query.
    /// Usually used to populate a `QueryIterationCursor`
    fn iteration_data<'s, 'a: 's, D: QueryData, F: QueryFilter>(
        &'a self,
        query: &QueryState<D, F, Self>,
        world: UnsafeWorldCell,
    ) -> IterationData<'s>;

    /// Returns true if the cache contains information about the archetype being matches by the query
    ///
    /// # Safety
    /// `archetype` must be from the `World` the state was initialized from.
    unsafe fn contains<D: QueryData, F: QueryFilter>(
        &self,
        query: &QueryState<D, F, Self>,
        archetype: &Archetype,
    ) -> bool;

    /// Creates a new [`QueryCache`] but does not populate it with the matched results from the World yet
    fn from_world_uninitialized<D: QueryData, F: QueryFilter>(world: &World) -> Self;

    /// Creates a new [`QueryCache`] but does not populate it with the matched results from the World yet
    fn from_builder_uninitialized<D: QueryData, F: QueryFilter>(
        builder: &QueryBuilder<D, F>,
    ) -> Self;

    /// Update the [`QueryCache`] by storing in the cache every new archetypes that match the query.
    fn update_archetypes<D: QueryData, F: QueryFilter>(
        &mut self,
        uncached: &UncachedQueryBorrow<'_, D, F>,
        world: UnsafeWorldCell,
    );

    /// Return a new cache that contains the archetypes matched by the intersection of itself and the
    /// other cache.
    fn join(&self, other: &Self) -> Self;
}

/// Contains a list of matches tables or archetypes, that can be used to iterate through archetypes
/// that match a query
#[derive(Clone)]
#[doc(hidden)]
pub struct IterationData<'s> {
    pub(super) is_dense: bool,
    pub(super) storage_ids: Cow<'s, [StorageId]>,
}

/// Default [`QueryCache`] to use if caching is enabled for a query.
/// Will store a pre-computed list of archetypes or tables that match a query.
#[derive(Clone)]
pub struct CacheState {
    pub(crate) archetype_generation: ArchetypeGeneration,
    /// Metadata about the [`Table`](crate::storage::Table)s matched by this query.
    pub(crate) matched_tables: FixedBitSet,
    /// Metadata about the [`Archetype`]s matched by this query.
    pub(crate) matched_archetypes: FixedBitSet,
    // NOTE: we maintain both a bitset and a vec because iterating the vec is faster
    pub(super) matched_storage_ids: Vec<StorageId>,
    // Represents whether this query iteration is dense or not. When this is true
    // `matched_storage_ids` stores `TableId`s, otherwise it stores `ArchetypeId`s.
    pub(super) is_dense: bool,
}

impl QueryCache for CacheState {
    fn iteration_data<'s, 'a: 's, D: QueryData, F: QueryFilter>(
        &'a self,
        _: &QueryState<D, F, Self>,
        _: UnsafeWorldCell,
    ) -> IterationData<'s> {
        IterationData {
            storage_ids: Cow::Borrowed(&self.matched_storage_ids),
            is_dense: self.is_dense,
        }
    }

    unsafe fn contains<D: QueryData, F: QueryFilter>(
        &self,
        _: &QueryState<D, F, Self>,
        archetype: &Archetype,
    ) -> bool {
        self.matched_archetypes.contains(archetype.id().index())
    }

    fn from_world_uninitialized<D: QueryData, F: QueryFilter>(_: &World) -> Self {
        // For queries without dynamic filters the dense-ness of the query is equal to the dense-ness
        // of its static type parameters.
        let is_dense = D::IS_DENSE && F::IS_DENSE;
        Self {
            archetype_generation: ArchetypeGeneration::initial(),
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            matched_storage_ids: Vec::new(),
            is_dense,
        }
    }

    fn from_builder_uninitialized<D: QueryData, F: QueryFilter>(
        builder: &QueryBuilder<D, F>,
    ) -> Self {
        // For dynamic queries the dense-ness is given by the query builder.
        let is_dense = builder.is_dense();
        Self {
            archetype_generation: ArchetypeGeneration::initial(),
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            matched_storage_ids: Vec::new(),
            is_dense,
        }
    }
    fn update_archetypes<D: QueryData, F: QueryFilter>(
        &mut self,
        uncached: &UncachedQueryBorrow<'_, D, F>,
        world: UnsafeWorldCell,
    ) {
        uncached.validate_world(world.id());
        if self.archetype_generation == world.archetypes().generation() {
            // skip if we are already up to date
            return;
        }
        let old_generation = core::mem::replace(
            &mut self.archetype_generation,
            world.archetypes().generation(),
        );
        uncached.iter_archetypes(old_generation, world.archetypes(), |archetype| {
            self.cache_archetype(archetype);
        });
    }

    fn join(&self, other: &Self) -> Self {
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
        CacheState {
            archetype_generation: self.archetype_generation,
            matched_tables,
            matched_archetypes,
            matched_storage_ids,
            is_dense,
        }
    }
}

impl Debug for CacheState {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CacheState")
            .field("matched_table_count", &self.matched_tables.count_ones(..))
            .field(
                "matched_archetype_count",
                &self.matched_archetypes.count_ones(..),
            )
            .finish()
    }
}

impl CacheState {
    /// Returns the tables matched by this query.
    pub fn matched_tables(&self) -> impl Iterator<Item = TableId> + '_ {
        self.matched_tables.ones().map(TableId::from_usize)
    }

    /// Returns the archetypes matched by this query.
    pub fn matched_archetypes(&self) -> impl Iterator<Item = ArchetypeId> + '_ {
        self.matched_archetypes.ones().map(ArchetypeId::new)
    }

    /// Add a new archetype in the cache
    fn cache_archetype(&mut self, archetype: &Archetype) {
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

/// [`QueryCache`] used if caching is disabled for a query.
///
/// We will not cache any matching archetypes for a query, so they will have to be recomputed
/// from scratch every time.
#[derive(Debug, Clone)]
pub struct Uncached {
    pub(super) is_dense: bool,
}

impl QueryCache for Uncached {
    fn iteration_data<'s, 'a: 's, D: QueryData, F: QueryFilter>(
        &'a self,
        query: &QueryState<D, F, Self>,
        world: UnsafeWorldCell,
    ) -> IterationData<'s> {
        let mut storage_ids = Vec::new();
        let mut matched_storages = FixedBitSet::new();
        query.iter_archetypes(
            ArchetypeGeneration::initial(),
            world.archetypes(),
            |archetype| {
                if let Some(storage_id) = if self.is_dense {
                    let table_index = archetype.table_id().as_usize();
                    (!matched_storages.contains(table_index)).then(|| {
                        matched_storages.grow_and_insert(table_index);
                        StorageId {
                            table_id: archetype.table_id(),
                        }
                    })
                } else {
                    let archetype_index = archetype.id().index();
                    (!matched_storages.contains(archetype_index)).then(|| {
                        matched_storages.grow_and_insert(archetype_index);
                        StorageId {
                            archetype_id: archetype.id(),
                        }
                    })
                } {
                    storage_ids.push(storage_id);
                }
            },
        );
        IterationData {
            is_dense: self.is_dense,
            storage_ids: Cow::Owned(storage_ids),
        }
    }

    unsafe fn contains<D: QueryData, F: QueryFilter>(
        &self,
        query: &QueryState<D, F, Self>,
        archetype: &Archetype,
    ) -> bool {
        // SAFETY: satisfied from QueryCache::contains's safety constraints
        unsafe { query.as_uncached().matches_archetype(archetype) }
    }

    fn from_world_uninitialized<D: QueryData, F: QueryFilter>(_: &World) -> Self {
        // For queries without dynamic filters the dense-ness of the query is equal to the dense-ness
        // of its static type parameters.
        Uncached {
            is_dense: D::IS_DENSE && F::IS_DENSE,
        }
    }

    fn from_builder_uninitialized<D: QueryData, F: QueryFilter>(
        builder: &QueryBuilder<D, F>,
    ) -> Self {
        Uncached {
            is_dense: builder.is_dense(),
        }
    }

    fn update_archetypes<D: QueryData, F: QueryFilter>(
        &mut self,
        uncached: &UncachedQueryBorrow<D, F>,
        world: UnsafeWorldCell,
    ) {
        uncached.validate_world(world.id());
    }

    fn join(&self, _: &Self) -> Self {
        self.clone()
    }
}
