use crate::{
    archetype::{ArchetypeId, Archetypes},
    component::Tick,
    entity::{Entities, Entity},
    storage::Tables,
    world::unsafe_world_cell::UnsafeWorldCell,
};

use super::{DebugCheckedUnwrap, QueryData, QueryEntityError, QueryFilter, QueryState};

/// Used for quickly fetching query item from a specific [`Entity`].
/// It caches the last fetch which could potentially be more efficient when dealing with many entities of the same archetype.
///
/// This struct is created by the [`Query::getter`](crate::system::Query::entity_getter) and [`Query::entity_getter_mut`](crate::system::Query::entity_getter_mut) methods.
pub struct PointQuery<'w, 's, D: QueryData, F: QueryFilter = ()> {
    // SAFETY: Must have access to the components registered in `state`.
    entities: &'w Entities,
    tables: &'w Tables,
    archetypes: &'w Archetypes,
    state: &'s QueryState<D, F>,
    fetch: D::Fetch<'w>,
    filter: F::Fetch<'w>,
    last_archetype_id: ArchetypeId,
}

impl<'w, 's, D: QueryData, F: QueryFilter> PointQuery<'w, 's, D, F> {
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
        PointQuery {
            state: query_state,
            entities: world.entities(),
            archetypes: world.archetypes(),
            // SAFETY: We only access table data that has been registered in `query_state`.
            // This means `world` has permission to access the data we use.
            tables: &world.storages().tables,
            fetch,
            filter,
            last_archetype_id: ArchetypeId::INVALID,
        }
    }

    /// Returns the read-only query item for the given [`Entity`].
    ///
    /// In case of a nonexisting entity or mismatched component, a [`QueryEntityError`] is returned instead.
    ///
    /// This is always guaranteed to run in `O(1)` time.
    #[inline]
    pub fn get(&mut self, entity: Entity) -> Result<D::Item<'_>, QueryEntityError> {
        let location = self
            .entities
            .get(entity)
            .ok_or(QueryEntityError::NoSuchEntity(entity))?;
        if !D::IS_DENSE || self.last_archetype_id != location.archetype_id {
            if !self
                .state
                .matched_archetypes
                .contains(location.archetype_id.index())
            {
                return Err(QueryEntityError::QueryDoesNotMatch(entity));
            }
            self.last_archetype_id = location.archetype_id;
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

    /// Returns the query item for the given [`Entity`].
    ///
    /// This is always guaranteed to run in `O(1)` time.
    ///
    /// # Safety
    ///
    /// `entity` must be valid and not pending.
    /// `entity` must on the same `World` that the `Query` was generated.
    /// `entity` must match the `Query` that generate this getter.
    #[inline]
    pub unsafe fn get_unchecked(&mut self, entity: Entity) -> D::Item<'_> {
        let location = self.entities.get_unchecked(entity);
        if !D::IS_DENSE || self.last_archetype_id != location.archetype_id {
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
            self.last_archetype_id = location.archetype_id;
        }
        D::fetch(&mut self.fetch, entity, location.table_row)
    }
}
