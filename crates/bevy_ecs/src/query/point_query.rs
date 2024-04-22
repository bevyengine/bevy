use crate::{
    archetype::{ArchetypeId, Archetypes},
    component::Tick,
    entity::{Entities, Entity},
    storage::{TableId, Tables},
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{DebugCheckedUnwrap, QueryData, QueryEntityError, QueryFilter, QueryState, StorageId};

/// Used for quickly fetching query item from a specific [`Entity`].
/// It caches the last fetch which could potentially be more efficient when dealing with many entities of the same archetype.
///
/// This struct is created by the [`Query::as_point_query`](crate::system::Query::as_point_query) and [`Query::as_point_query_mut`](crate::system::Query::as_point_query_mut) methods.
pub struct PointQuery<'w, 's, D: QueryData, F: QueryFilter = ()> {
    // SAFETY: Must have access to the components registered in `state`.
    entities: &'w Entities,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    state: &'s QueryState<D, F>,
    fetch: D::Fetch<'w>,
    filter: F::Fetch<'w>,
    last_storage_id: StorageId,
}

impl<'w, 's, D: QueryData, F: QueryFilter> PointQuery<'w, 's, D, F> {
    const IS_DENSE: bool = D::IS_DENSE && F::IS_DENSE;
    /// # Safety
    /// - `world` must have permission to access any of the components registered in `query_state`.
    /// - `world` must be the same one used to initialize `query_state`.
    #[inline]
    pub(crate) unsafe fn new(
        world: UnsafeWorldCell<'w>,
        query_state: &'s QueryState<D, F>,
        last_run: Tick,
        this_run: Tick,
    ) -> PointQuery<'w, 's, D, F> {
        let fetch = D::init_fetch(world, &query_state.fetch_state, last_run, this_run);
        let filter = F::init_fetch(world, &query_state.filter_state, last_run, this_run);
        let last_storage_id = if Self::IS_DENSE {
            StorageId {
                table_id: TableId::INVALID,
            }
        } else {
            StorageId {
                archetype_id: ArchetypeId::INVALID,
            }
        };

        PointQuery {
            state: query_state,
            entities: world.entities(),
            archetypes: world.archetypes(),
            // SAFETY: We only access table data that has been registered in `query_state`.
            // This means `world` has permission to access the data we use.
            tables: &world.storages().tables,
            fetch,
            filter,
            last_storage_id,
        }
    }

    /// Returns the read-only query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get(&mut self, entity: Entity) -> Result<D::Item<'_>, QueryEntityError> {
        // SAFETY: system runs without conflicts with other systems.
        // same-system queries have runtime borrow checks when they conflict
        unsafe { std::mem::transmute(self.get_unsafe(entity)) }
    }

    /// Returns the query item for the given [`Entity`] without any check.
    ///
    /// WARNING: This method should only be recommended in extreme cases where performance is the highest priority.
    ///
    /// For a safe alternative see [`Self::get`].
    ///
    /// # Safety
    ///
    /// `entity` must be valid and not in a pending state.
    /// `entity` must be within the same `World` which the `Query` was generated.
    /// `entity` must match the scope of `Query`.
    #[inline]
    pub unsafe fn get_unchecked(&mut self, entity: Entity) -> D::Item<'_> {
        let location = self.entities.get(entity).debug_checked_unwrap();
        if Self::IS_DENSE {
            if self.last_storage_id.table_id != location.table_id {
                // SAFETY: `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                // `table` is from the world that `fetch/filter` were created for,
                // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                let table = self.tables.get(location.table_id).debug_checked_unwrap();
                D::set_table(&mut self.fetch, &self.state.fetch_state, table);
                F::set_table(&mut self.filter, &self.state.filter_state, table);
                self.last_storage_id.table_id = location.table_id;
            }
        } else if self.last_storage_id.archetype_id != location.archetype_id {
            // SAFETY: `archetype` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            // `table` is from the world that `fetch/filter` were created for,
            // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
            let archetype = self
                .archetypes
                .get(location.archetype_id)
                .debug_checked_unwrap();
            let table = self.tables.get(location.table_id).debug_checked_unwrap();
            D::set_archetype(&mut self.fetch, &self.state.fetch_state, archetype, table);
            F::set_archetype(&mut self.filter, &self.state.filter_state, archetype, table);
            self.last_storage_id.archetype_id = location.archetype_id;
        }

        let item = D::fetch(&mut self.fetch, entity, location.table_row);
        std::mem::transmute(item)
    }

    /// Returns the query item for the given [`Entity`].
    ///
    /// # Safety
    ///
    /// This function makes it possible to violate Rust's aliasing guarantees.
    /// You must make sure this call does not result in multiple mutable references to the same component.
    #[inline]
    pub unsafe fn get_unsafe(&mut self, entity: Entity) -> Result<D::Item<'w>, QueryEntityError> {
        let location = self
            .entities
            .get(entity)
            .ok_or(QueryEntityError::NoSuchEntity(entity))?;

        if Self::IS_DENSE {
            // SAFETY: only accessed by dense query
            if unsafe { self.last_storage_id.table_id != location.table_id } {
                if !self
                    .state
                    .matched_tables
                    .contains(location.table_id.as_usize())
                {
                    return Err(QueryEntityError::QueryDoesNotMatch(entity));
                }
                // SAFETY: `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                // `table` is from the world that `fetch/filter` were created for,
                // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                unsafe {
                    let table = self.tables.get(location.table_id).debug_checked_unwrap();
                    D::set_table(&mut self.fetch, &self.state.fetch_state, table);
                    F::set_table(&mut self.filter, &self.state.filter_state, table);
                }
                self.last_storage_id.table_id = location.table_id;
            }
        } else {
            // SAFETY: only accessed by none dense query
            if unsafe { self.last_storage_id.archetype_id != location.archetype_id } {
                if !self
                    .state
                    .matched_archetypes
                    .contains(location.archetype_id.index())
                {
                    return Err(QueryEntityError::QueryDoesNotMatch(entity));
                }
                // SAFETY: `archetype` is from the world that `fetch/filter` were created for,
                // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                // `table` is from the world that `fetch/filter` were created for,
                // `fetch_state`/`filter_state` are the states that `fetch/filter` were initialized with
                unsafe {
                    let archetype = self
                        .archetypes
                        .get(location.archetype_id)
                        .debug_checked_unwrap();
                    let table = self.tables.get(location.table_id).debug_checked_unwrap();
                    D::set_archetype(&mut self.fetch, &self.state.fetch_state, archetype, table);
                    F::set_archetype(&mut self.filter, &self.state.filter_state, archetype, table);
                }
                self.last_storage_id.archetype_id = location.archetype_id;
            }
        }
        // SAFETY: `filter` and `fetch` have been configured correctly.
        unsafe {
            if F::filter_fetch(&mut self.filter, entity, location.table_row) {
                Ok(D::fetch(&mut self.fetch, entity, location.table_row))
            } else {
                Err(QueryEntityError::QueryDoesNotMatch(entity))
            }
        }
    }
}
