//! Types for entity disabling.
//!
//! Disabled entities do not show up in queries unless the query explicitly mentions them.
//!
//! If for example we have `Disabled` as an entity disabling component, when you add `Disabled`
//! to an entity, the entity will only be visible to queries with a filter like
//! [`With`]`<Disabled>` or query data like [`Has`]`<Disabled>`.
//!
//! ### Note
//!
//! Currently only queries for which the cache is built after enabling a filter will have entities
//! with those components filtered. As a result, they should generally only be modified before the
//! app starts.
//!
//! Because filters are applied to all queries they can have performance implication for
//! the enire [`World`], especially when they cause queries to mix sparse and table components.
//! See [`Query` performance] for more info.
//!
//! [`With`]: crate::prelude::With
//! [`Has`]: crate::prelude::Has
//! [`World`]: crate::prelude::World
//! [`Query` performance]: crate::prelude::Query#performance

use crate::{
    component::{ComponentId, Components, StorageType},
    query::FilteredAccess,
};
use bevy_ecs_macros::{Component, Resource};

#[cfg(feature = "bevy_reflect")]
use {crate::reflect::ReflectComponent, bevy_reflect::Reflect};

/// A marker component for disabled entities. See [the module docs] for more info.
///
/// [the module docs]: crate::entity_disabling
#[derive(Component)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Component))]
pub struct Disabled;

/// The default filters for all queries, these are used to globally exclude entities from queries.
/// See the [module docs](crate::entity_disabling) for more info.
#[derive(Resource, Default, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct DefaultQueryFilters {
    disabled: Option<ComponentId>,
}

impl DefaultQueryFilters {
    /// Set the [`ComponentId`] for the entity disabling marker
    pub(crate) fn set_disabled(&mut self, component_id: ComponentId) -> Option<()> {
        if self.disabled.is_some() {
            return None;
        }
        self.disabled = Some(component_id);
        Some(())
    }

    /// Get an iterator over all currently enabled filter components
    pub fn ids(&self) -> impl Iterator<Item = ComponentId> {
        [self.disabled].into_iter().flatten()
    }

    pub(super) fn apply(&self, component_access: &mut FilteredAccess<ComponentId>) {
        for component_id in self.ids() {
            if !component_access.contains(component_id) {
                component_access.and_without(component_id);
            }
        }
    }

    pub(super) fn is_dense(&self, components: &Components) -> bool {
        self.ids().all(|component_id| {
            components
                .get_info(component_id)
                .is_some_and(|info| info.storage_type() == StorageType::Table)
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use alloc::{vec, vec::Vec};

    #[test]
    fn test_set_filters() {
        let mut filters = DefaultQueryFilters::default();
        assert_eq!(0, filters.ids().count());

        assert!(filters.set_disabled(ComponentId::new(1)).is_some());
        assert!(filters.set_disabled(ComponentId::new(3)).is_none());

        assert_eq!(1, filters.ids().count());
        assert_eq!(Some(ComponentId::new(1)), filters.ids().next());
    }

    #[test]
    fn test_apply_filters() {
        let mut filters = DefaultQueryFilters::default();
        filters.set_disabled(ComponentId::new(1));

        // A component access with an unrelated component
        let mut component_access = FilteredAccess::<ComponentId>::default();
        component_access
            .access_mut()
            .add_component_read(ComponentId::new(2));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(0, applied_access.with_filters().count());
        assert_eq!(
            vec![ComponentId::new(1)],
            applied_access.without_filters().collect::<Vec<_>>()
        );

        // We add a with filter, now we expect to see both filters
        component_access.and_with(ComponentId::new(4));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(
            vec![ComponentId::new(4)],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert_eq!(
            vec![ComponentId::new(1)],
            applied_access.without_filters().collect::<Vec<_>>()
        );

        let copy = component_access.clone();
        // We add a rule targeting a default component, that filter should no longer be added
        component_access.and_with(ComponentId::new(1));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(
            vec![ComponentId::new(1), ComponentId::new(4)],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert_eq!(0, applied_access.without_filters().count());

        // Archetypal access should also filter rules
        component_access = copy.clone();
        component_access
            .access_mut()
            .add_archetypal(ComponentId::new(1));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(
            vec![ComponentId::new(4)],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert_eq!(0, applied_access.without_filters().count());
    }
}
