use crate as bevy_ecs;
use crate::{component::ComponentId, query::FilteredAccess};
use bevy_ecs_macros::Resource;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum FilterKind {
    With,
    Without,
}

#[derive(PartialEq, Eq, Debug)]
struct DefaultFilter {
    component_id: ComponentId,
    kind: FilterKind,
}

/// A list of default query filters, these can be used to globally exclude entities from queries.
/// Each individual filter is only applied to queries that don't mention that component.
///
/// These filters are only applied to queries initialized after updating this resource,
/// it should most likely only be modified before the app starts.
#[derive(Resource, Default)]
pub struct DefaultQueryFilters(Vec<DefaultFilter>);

impl DefaultQueryFilters {
    /// Add a With filter to the default query filters
    pub fn with(&mut self, component_id: ComponentId) {
        self.set(component_id, FilterKind::With);
    }

    /// Add a Without filter to the default query filters
    pub fn without(&mut self, component_id: ComponentId) {
        self.set(component_id, FilterKind::Without);
    }

    /// Remove a filter for the specified [`ComponentId`]
    pub fn remove(&mut self, component_id: ComponentId) {
        self.0.retain(|filter| filter.component_id != component_id);
    }

    fn set(&mut self, component_id: ComponentId, kind: FilterKind) {
        if let Some(filter) = self
            .0
            .iter_mut()
            .find(|filter| filter.component_id == component_id)
        {
            filter.kind = kind;
        } else {
            self.0.push(DefaultFilter { component_id, kind });
        }
    }

    pub(super) fn apply(&self, component_access: &mut FilteredAccess<ComponentId>) {
        for filter in self.0.iter() {
            if !contains_component(component_access, filter.component_id) {
                match filter.kind {
                    FilterKind::With => component_access.and_with(filter.component_id),
                    FilterKind::Without => component_access.and_without(filter.component_id),
                }
            }
        }
    }
}

fn contains_component(
    component_access: &FilteredAccess<ComponentId>,
    component_id: ComponentId,
) -> bool {
    component_access.access().has_read(component_id)
        || component_access.access().has_archetypal(component_id)
        || component_access.filter_sets.iter().any(|f| {
            f.with.contains(component_id.index()) || f.without.contains(component_id.index())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_filters() {
        let mut filters = DefaultQueryFilters::default();
        filters.with(ComponentId::new(1));
        filters.with(ComponentId::new(3));
        filters.without(ComponentId::new(3));

        assert_eq!(2, filters.0.len());
        assert_eq!(
            DefaultFilter {
                component_id: ComponentId::new(1),
                kind: FilterKind::With
            },
            filters.0[0]
        );
        assert_eq!(
            DefaultFilter {
                component_id: ComponentId::new(3),
                kind: FilterKind::Without
            },
            filters.0[1]
        );
    }

    #[test]
    fn test_apply_filters() {
        let mut filters = DefaultQueryFilters::default();
        filters.with(ComponentId::new(1));
        filters.without(ComponentId::new(3));

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
