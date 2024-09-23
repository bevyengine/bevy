use crate as bevy_ecs;
use crate::{
    component::{ComponentId, Components, StorageType},
    query::FilteredAccess,
};
use bevy_ecs_macros::Resource;

/// The default filters for all queries, these are used to globally exclude entities from queries.
/// Default filters are applied to any queries that does not explicitly mention that component.
///
/// If for example the component `Disabled` is registered here, entities with this component will
/// only be visible to a [`Query`](crate::prelude::Query) containing something like
/// [`With<Disabled>`](crate::prelude::With) or [`Has<Disabled>`](crate::prelude::Has).
/// See below for a more detailed example.
///
/// These filters are only applied to queries whose cache is generated after updating this resource,
/// As a result, this resource should generally only be modified before the app starts (typically
/// during plugin construction)
///
/// Because these filters are applied to all queries, the storage type of the component has
/// implications for the entire app. See [`Query` performance] for more info.
///
/// ### Example
///
/// ```rust
/// # use bevy_ecs::{prelude::*, query::DefaultQueryFilters};
/// # #[derive(Component)]
/// # struct Disabled;
/// let mut world = World::new();
/// let mut filters = DefaultQueryFilters::default();
/// filters.set_disabled(world.init_component::<Disabled>());
/// world.insert_resource(filters);
///
/// // This entity is not Disabled, so most queries will see it
/// let entity_a = world.spawn_empty().id();
///
/// // This entity has Disabled, so most queries won't see it
/// let entity_b = world.spawn(Disabled).id();
///
/// // This query does not mention either of the markers, so it only gets entity_a
/// let mut query = world.query::<Entity>();
/// assert_eq!(1, query.iter(&world).count());
/// assert_eq!(entity_a, query.get_single(&world).unwrap());
///
/// // This query only wants entities that are Disabled, thus it only sees entity_b
/// let mut query = world.query_filtered::<Entity, With<Disabled>>();
/// assert_eq!(1, query.iter(&world).count());
/// assert_eq!(entity_b, query.get_single(&world).unwrap());
///
/// // This also works for query data
/// let mut query = world.query::<(Entity, Has<Disabled>)>();
/// assert_eq!(2, query.iter(&world).count());
/// assert_eq!(vec![(entity_a, false), (entity_b, true)], query.iter(&world).collect::<Vec<_>>());
/// ```
///
/// [`Query` performance]: crate::prelude::Query#performance
#[derive(Resource, Default, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct DefaultQueryFilters {
    disabled: Option<ComponentId>,
}

impl DefaultQueryFilters {
    /// Set the [`ComponentId`] for the entity disabling marker
    pub fn set_disabled(&mut self, component_id: ComponentId) -> Option<()> {
        if self.disabled.is_some() {
            return None;
        }
        self.disabled = Some(component_id);
        Some(())
    }

    /// Get an iterator over all default filter components
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
                .map_or(false, |info| info.storage_type() == StorageType::Table)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
