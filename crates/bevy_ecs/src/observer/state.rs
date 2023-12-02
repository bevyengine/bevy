use super::*;

// State for [`Query`] like behaviour for [`Observer`]
// Will be unified with [`QueryState`] with queries as entities
#[derive(Component)]
pub(crate) struct ObserverState<Q: WorldQueryData, F: WorldQueryFilter> {
    pub(crate) fetch_state: Q::State,
    pub(crate) filter_state: F::State,
    pub(crate) component_access: FilteredAccess<ComponentId>,
    pub(crate) last_event_id: u32,
}

impl<Q: WorldQueryData, F: WorldQueryFilter> ObserverState<Q, F> {
    pub(crate) fn new(world: &mut World) -> Self {
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

        Self {
            fetch_state,
            filter_state,
            component_access,
            last_event_id: 0,
        }
    }
}
