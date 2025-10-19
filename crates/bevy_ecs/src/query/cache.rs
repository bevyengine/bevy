use core::fmt::{Debug, Formatter};
use alloc::vec::Vec;
use fixedbitset::FixedBitSet;
use log::warn;
use bevy_ecs::archetype::{Archetype, ArchetypeGeneration, ArchetypeId, Archetypes};
use bevy_ecs::component::ComponentId;
use bevy_ecs::entity_disabling::DefaultQueryFilters;
use bevy_ecs::prelude::World;
use bevy_ecs::query::{QueryData, QueryFilter};
use bevy_ecs::query::state::StorageId;
use bevy_ecs::storage::{SparseSetIndex, TableId};
use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
use crate::query::QueryState;

impl<D: QueryData, F: QueryFilter, C: QueryCache> QueryState<D, F, C> {

    /// Splits self into an immutable view of the "prefix"
    /// (all fields *except* cache) and a mutable ref to the `cache`.
    pub fn split_cache(&mut self) -> (&QueryState<D, F, Uncached>, &mut C) {
        // This is safe because `QueryState<..., Uncached>` is a
        // valid "prefix" of `QueryState<..., C>`, and QueryState uses `repr(c)`
        let rest: &QueryState<D, F, Uncached> = unsafe {
            &*(self as *mut Self as *const QueryState<D, F, Uncached>)
        };

        // This is safe because `cache` is disjoint from the prefix.
        let cache_mut: &mut C = &mut self.cache;

        (rest, cache_mut)
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
    pub fn update_archetypes(&mut self, world: &World) {
        self.update_archetypes_unsafe_world_cell(world.as_unsafe_world_cell_readonly());
    }

    pub fn update_archetypes_unsafe_world_cell(&mut self, world: UnsafeWorldCell) {
        let (uncached_state, cache) = self.split_cache();
        cache.update_archetypes(uncached_state, world);
    }

    /// Safety: todo.
    pub(crate) fn matches(&self, archetype: &Archetype) -> bool {
        // SAFETY: from parent function's safety constraint
        unsafe { self.cache.contains(self, archetype) }
    }
}


impl<D: QueryData, F: QueryFilter> QueryState<D, F, Uncached> {
    /// Returns `true` if this query matches a set of components. Otherwise, returns `false`.
    ///
    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    unsafe fn matches_archetype(&self, archetype: &Archetype) -> bool {
        D::matches_component_set(&self.fetch_state, &|id| archetype.contains(id))
            && F::matches_component_set(&self.filter_state, &|id| archetype.contains(id))
            && self.matches_component_set(&|id| archetype.contains(id))
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

    /// Iterate through all archetypes that match the [`QueryState`] with an [`ArchetypeGeneration`] higher than the provided one,
    /// and call `f` on each of them.
    fn iter_archetypes(&self, archetype_generation: ArchetypeGeneration, archetypes: &Archetypes, mut f: impl FnMut(&Archetype)) {
        if self.component_access.required.is_empty() {
            archetypes[archetype_generation..].iter().for_each(|archetype| {
                // SAFETY: The validate_world call ensures that the world is the same the QueryState
                // was initialized from.
                if unsafe { self.matches_archetype(archetype) }  {
                    f(archetype);
                }
            })
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
                        f(archetype)
                    }
                }
            }
        }
    }

}

pub trait QueryCache: Debug + Clone {

    fn iteration_data<'s, D: QueryData, F: QueryFilter>(&self, query: &QueryState<D, F, Self>, world: UnsafeWorldCell, storage_ids: &'s mut Vec<StorageId>) -> IterationData<'s>;

    /// # Safety
    /// `archetype` must be from the `World` this state was initialized from.
    unsafe fn contains<D: QueryData, F: QueryFilter>(&self, query: &QueryState<D, F, Self>, archetype: &Archetype) -> bool;

    /// Creates a new [`QueryCache`] but does not populate it with the matched results from the World yet
    ///
    /// `new_archetype` and its variants must be called on all of the World's archetypes before the
    /// state can return valid query results.
    fn uninitialized<D: QueryData, F: QueryFilter>(world: &World) -> Self;

    fn update_archetypes<D: QueryData, F: QueryFilter>(&mut self, uncached: &QueryState<D, F, Uncached>, world: UnsafeWorldCell);

    /// Return a new cache that contains the archetypes matched by the query joining self and other
    fn join(&self, other: &Self) -> Self;


}

#[derive(Clone)]
pub(super) struct IterationData<'s> {
    pub(super) is_dense: bool,
    pub(super) storage_ids: core::slice::Iter<'s, StorageId>,
}

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

    fn iteration_data<'s, D: QueryData, F: QueryFilter>(self: &Self, _: &QueryState<D, F, Self>, _: UnsafeWorldCell, _: &'s mut Vec<StorageId>) -> IterationData<'s> {
        IterationData {
            storage_ids: self.matched_storage_ids.iter(),
            is_dense: self.is_dense,
        }
    }

    unsafe fn contains<D: QueryData, F: QueryFilter>(&self, _: &QueryState<D, F, Self>, archetype: &Archetype) -> bool {
        self.matched_archetypes.contains(archetype.id().index())
    }

    fn uninitialized<D: QueryData, F: QueryFilter>(world: &World) -> Self {
        // For queries without dynamic filters the dense-ness of the query is equal to the dense-ness
        // of its static type parameters.
        let mut is_dense = D::IS_DENSE && F::IS_DENSE;

        if let Some(default_filters) = world.get_resource::<DefaultQueryFilters>() {
            is_dense &= default_filters.is_dense(world.components());
        }

        Self {
            archetype_generation: ArchetypeGeneration::initial(),
            matched_tables: Default::default(),
            matched_archetypes: Default::default(),
            matched_storage_ids: Vec::new(),
            is_dense,
        }
    }
    fn update_archetypes<D: QueryData, F: QueryFilter>(&mut self, uncached: &QueryState<D, F, Uncached>, world: UnsafeWorldCell) {
        uncached.validate_world(world.id());
        if self.archetype_generation == world.archetypes().generation() {
            // skip if we are already up to date
            return
        }
        let old_generation =
                core::mem::replace(&mut self.archetype_generation, world.archetypes().generation());
        uncached.iter_archetypes(old_generation, world.archetypes(), |archetype| self.cache_archetype(archetype));
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


#[derive(Debug, Clone)]
pub struct Uncached;



impl QueryCache for Uncached {

    fn iteration_data<'s, D: QueryData, F: QueryFilter>(&self, query: &QueryState<D, F, Self>, world: UnsafeWorldCell, storage_ids: &'s mut Vec<StorageId>) -> IterationData<'s> {
        let generation = world.archetypes().generation();
        // TODO: what about computing is_dense from DefaultQueryFilters?
        //  Realistically all DefaultQueryFilters would be dense. We should enforce it.
        let is_dense = D::IS_DENSE && F::IS_DENSE;
        query.iter_archetypes(generation, world.archetypes(), |archetype| {
            storage_ids.push(if !is_dense {
                StorageId { archetype_id: archetype.id() }
            } else {
                StorageId { table_id: archetype.table_id() }
            })
        });
        IterationData {
            is_dense,
            storage_ids: storage_ids.iter()
        }
    }

    unsafe fn contains<D: QueryData, F: QueryFilter>(&self, query: &QueryState<D, F, Self>, archetype: &Archetype) -> bool {
        // SAFETY: satisfied from QueryCache::contains's safety constraints
        unsafe { query.matches_archetype(archetype) }
    }

    fn uninitialized<D: QueryData, F: QueryFilter>(_: &World) -> Self {
        Uncached
    }

    /// We do not update anything. This is here for feature parity.
    fn update_archetypes<D: QueryData, F: QueryFilter>(&mut self, _: &QueryState<D, F, Uncached>, _: UnsafeWorldCell) {
    }

    fn join(&self, _: &Self) -> Self {
        self.clone()
    }
}