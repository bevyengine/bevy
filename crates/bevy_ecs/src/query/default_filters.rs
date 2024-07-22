use crate as bevy_ecs;
use crate::{component::ComponentId, query::FilteredAccess};
use bevy_ecs_macros::Resource;
use bevy_utils::HashMap;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
enum FilterKind {
    With,
    Without,
}

/// A list of default query filters, these can be used to globally exclude entities from queries.
/// Default filters are applied to any queries that does not explicitly mention that component.
///
/// If for example we register a `Hidden` component using the `without_untyped` method, entities
/// with this component will only be visible to a [`Query`](crate::prelude::Query) containing
/// something like [`With<Hidden>`](crate::prelude::With) or [`Has<Hidden>`](crate::prelude::Has).
/// See below for a more detailed example.
///
/// These filters are only applied to queries whose cache is generated after updating this resource,
/// As a result, this resource should generally only be modified before the app starts (typically
/// during plugin construction)
///
/// See `World::set_default_with_filter`, `World::set_default_without_filter`, and
/// `World::unset_default_filter` for easier access.
///
/// ### Example
///
/// ```rust
/// # use bevy_ecs::{prelude::*, query::DefaultQueryFilters};
/// # #[derive(Component)]
/// # struct Enabled;
/// # #[derive(Component)]
/// # struct TopSecret;
/// #
/// let mut world = World::new();
/// let mut filters = DefaultQueryFilters::default();
/// filters.with_untyped(world.init_component::<Enabled>());
/// filters.without_untyped(world.init_component::<TopSecret>());
/// world.insert_resource(filters);
///
/// // This entity is missing Enabled, so most queries won't see it
/// let entity_a = world.spawn_empty().id();
///
/// // This entity has Enabled, and isn't TopSecret, so most queries see it
/// let entity_b = world.spawn(Enabled).id();
///
/// // This entity is Enabled and TopSecret, so most queries won't see it
/// let entity_c = world.spawn((Enabled, TopSecret)).id();
///
/// // This entity is TopSecret but not enabled, so only very specific queries will see it
/// let entity_d = world.spawn(TopSecret).id();
///
/// // This query does not mention either of the markers, so it only gets entity_b
/// let mut query = world.query::<Entity>();
/// assert_eq!(1, query.iter(&world).count());
/// assert_eq!(entity_b, query.get_single(&world).unwrap());
///
/// // This query only wants entities that aren't Enabled, but can't see TopSecret entities,
/// // thus it only sees entity_a
/// let mut query = world.query_filtered::<Entity, Without<Enabled>>();
/// assert_eq!(1, query.iter(&world).count());
/// assert_eq!(entity_a, query.get_single(&world).unwrap());
///
/// // This query only wants TopSecret entities, but still can't see entities that aren't Enabled,
/// // thus it only sees entity_c
/// let mut query = world.query_filtered::<Entity, With<TopSecret>>();
/// assert_eq!(1, query.iter(&world).count());
/// assert_eq!(entity_c, query.get_single(&world).unwrap());
///
/// // This query mentions both, so it gets results as if the filters don't exist
/// let mut query = world.query::<(Entity, Has<Enabled>, Has<TopSecret>)>();
/// assert_eq!(4, query.iter(&world).count());
/// ```
#[derive(Resource, Default, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct DefaultQueryFilters(HashMap<ComponentId, FilterKind>);

impl DefaultQueryFilters {
    /// Add a With filter to the default query filters.
    /// Removes any Without filter for this component if present.
    pub fn with_untyped(&mut self, component_id: ComponentId) {
        self.0.insert(component_id, FilterKind::With);
    }

    /// Add a Without filter to the default query filters.
    /// Removes any With filter for this component if present.
    pub fn without_untyped(&mut self, component_id: ComponentId) {
        self.0.insert(component_id, FilterKind::Without);
    }

    /// Remove a filter for the specified [`ComponentId`]
    pub fn remove_untyped(&mut self, component_id: ComponentId) {
        self.0.remove(&component_id);
    }

    pub(super) fn apply(&self, component_access: &mut FilteredAccess<ComponentId>) {
        for (&component_id, kind) in self.0.iter() {
            if !component_access.contains(component_id) {
                match kind {
                    FilterKind::With => component_access.and_with(component_id),
                    FilterKind::Without => component_access.and_without(component_id),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_filters() {
        let mut filters = DefaultQueryFilters::default();
        filters.with_untyped(ComponentId::new(1));
        filters.with_untyped(ComponentId::new(3));
        filters.without_untyped(ComponentId::new(3));

        assert_eq!(2, filters.0.len());
        assert_eq!(Some(&FilterKind::With), filters.0.get(&ComponentId::new(1)));
        assert_eq!(
            Some(&FilterKind::Without),
            filters.0.get(&ComponentId::new(3))
        );
    }

    #[test]
    fn test_apply_filters() {
        let mut filters = DefaultQueryFilters::default();
        filters.with_untyped(ComponentId::new(1));
        filters.without_untyped(ComponentId::new(3));

        // A component access with an unrelated component
        let mut component_access = FilteredAccess::<ComponentId>::default();
        component_access.access_mut().add_read(ComponentId::new(2));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(
            vec![ComponentId::new(1)],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert_eq!(
            vec![ComponentId::new(3)],
            applied_access.without_filters().collect::<Vec<_>>()
        );

        // We add a with filter, now we expect to see both filters
        component_access.and_with(ComponentId::new(4));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(
            vec![ComponentId::new(1), ComponentId::new(4)],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert_eq!(
            vec![ComponentId::new(3)],
            applied_access.without_filters().collect::<Vec<_>>()
        );

        // We add a rule targeting a default component, that filter should no longer be added
        component_access.and_with(ComponentId::new(3));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(
            vec![
                ComponentId::new(1),
                ComponentId::new(3),
                ComponentId::new(4)
            ],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert!(applied_access.without_filters().next().is_none());

        // Archetypal access should also filter rules
        component_access
            .access_mut()
            .add_archetypal(ComponentId::new(1));

        let mut applied_access = component_access.clone();
        filters.apply(&mut applied_access);
        assert_eq!(
            vec![ComponentId::new(3), ComponentId::new(4)],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert!(applied_access.without_filters().next().is_none());
    }
}
