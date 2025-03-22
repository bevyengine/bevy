//! Disabled entities do not show up in queries unless the query explicitly mentions them.
//!
//! Entities which are disabled in this way are not removed from the [`World`],
//! and their relationships remain intact.
//! In many cases, you may want to disable entire trees of entities at once,
//! using [`EntityCommands::insert_recursive`](crate::prelude::EntityCommands::insert_recursive).
//!
//! While Bevy ships with a built-in [`Disabled`] component, you can also create your own
//! disabling components, which will operate in the same way but can have distinct semantics.
//!
//! ```
//! use bevy_ecs::prelude::*;
//!
//! // Our custom disabling component!
//! #[derive(Component, Clone)]
//! struct Prefab;
//!
//! #[derive(Component)]
//! struct A;
//!
//! let mut world = World::new();
//! world.register_disabling_component::<Prefab>();
//! world.spawn((A, Prefab));
//! world.spawn((A,));
//! world.spawn((A,));
//!
//! let mut normal_query = world.query::<&A>();
//! assert_eq!(2, normal_query.iter(&world).count());
//!
//! let mut prefab_query = world.query_filtered::<&A, With<Prefab>>();
//! assert_eq!(1, prefab_query.iter(&world).count());
//!
//! let mut maybe_prefab_query = world.query::<(&A, Has<Prefab>)>();
//! assert_eq!(3, maybe_prefab_query.iter(&world).count());
//! ```
//!
//! ## Default query filters
//!
//! In Bevy, entity disabling is implemented through the construction of a global "default query filter".
//! Queries which do not explicitly mention the disabled component will not include entities with that component.
//! If an entity has multiple disabling components, it will only be included in queries that mention all of them.
//!
//! For example, `Query<&Position>` will not include entities with the [`Disabled`] component,
//! even if they have a `Position` component,
//! but `Query<&Position, With<Disabled>>` or `Query<(&Position, Has<Disabled>)>` will see them.
//!
//! Entities with disabling components are still present in the [`World`] and can be accessed directly,
//! using methods on [`World`] or [`Commands`](crate::prelude::Commands).
//!
//! ### Warnings
//!
//! Currently, only queries for which the cache is built after enabling a default query filter will have entities
//! with those components filtered. As a result, they should generally only be modified before the
//! app starts.
//!
//! Because filters are applied to all queries they can have performance implication for
//! the enire [`World`], especially when they cause queries to mix sparse and table components.
//! See [`Query` performance] for more info.
//!
//! Custom disabling components can cause significant interoperability issues within the ecosystem,
//! as users must be aware of each disabling component in use.
//! Libraries should think carefully about whether they need to use a new disabling component,
//! and clearly communicate their presence to their users to avoid the new for library compatibility flags.
//!
//! [`With`]: crate::prelude::With
//! [`Has`]: crate::prelude::Has
//! [`World`]: crate::prelude::World
//! [`Query` performance]: crate::prelude::Query#performance

use crate::{
    component::{ComponentId, Components, StorageType},
    query::FilteredAccess,
    world::{FromWorld, World},
};
use bevy_ecs_macros::{Component, Resource};
use smallvec::SmallVec;

#[cfg(feature = "bevy_reflect")]
use {
    crate::reflect::ReflectComponent, bevy_reflect::std_traits::ReflectDefault,
    bevy_reflect::Reflect,
};

/// A marker component for disabled entities.
///
/// Semantically, this component is used to mark entities that are temporarily disabled (typically for gameplay reasons),
/// but will likely be re-enabled at some point.
///
/// Like all disabling components, this only disables the entity itself,
/// not its children or other entities that reference it.
/// To disable an entire tree of entities, use [`EntityCommands::insert_recursive`](crate::prelude::EntityCommands::insert_recursive).
///
/// Every [`World`] has a default query filter that excludes entities with this component,
/// registered in the [`DefaultQueryFilters`] resource.
/// See [the module docs] for more info.
///
/// [the module docs]: crate::entity_disabling
#[derive(Component, Clone, Debug, Default)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Component),
    reflect(Debug, Clone, Default)
)]
// This component is registered as a disabling component during World::bootstrap
pub struct Disabled;

/// Default query filters work by excluding entities with certain components from most queries.
///
/// If a query does not explicitly mention a given disabling component, it will not include entities with that component.
/// To be more precise, this checks if the query's [`FilteredAccess`] contains the component,
/// and if it does not, adds a [`Without`](crate::prelude::Without) filter for that component to the query.
///
/// This resource is initialized in the [`World`] whenever a new world is created,
/// with the [`Disabled`] component as a disabling component.
///
/// Note that you can remove default query filters by overwriting the [`DefaultQueryFilters`] resource.
/// This can be useful as a last resort escape hatch, but is liable to break compatibility with other libraries.
///
/// See the [module docs](crate::entity_disabling) for more info.
///
///
/// # Warning
///
/// Default query filters are a global setting that affects all queries in the [`World`],
/// and incur a small performance cost for each query.
///
/// They can cause significant interoperability issues within the ecosystem,
/// as users must be aware of each disabling component in use.
///
/// Think carefully about whether you need to use a new disabling component,
/// and clearly communicate their presence in any libraries you publish.
#[derive(Resource, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(bevy_reflect::Reflect))]
pub struct DefaultQueryFilters {
    // We only expect a few components per application to act as disabling components, so we use a SmallVec here
    // to avoid heap allocation in most cases.
    disabling: SmallVec<[ComponentId; 4]>,
}

impl FromWorld for DefaultQueryFilters {
    fn from_world(world: &mut World) -> Self {
        let mut filters = DefaultQueryFilters::empty();
        let disabled_component_id = world.register_component::<Disabled>();
        filters.register_disabling_component(disabled_component_id);
        filters
    }
}

impl DefaultQueryFilters {
    /// Creates a new, completely empty [`DefaultQueryFilters`].
    ///
    /// This is provided as an escape hatch; in most cases you should initialize this using [`FromWorld`],
    /// which is automatically called when creating a new [`World`].
    #[must_use]
    pub fn empty() -> Self {
        DefaultQueryFilters {
            disabling: SmallVec::new(),
        }
    }

    /// Adds this [`ComponentId`] to the set of [`DefaultQueryFilters`],
    /// causing entities with this component to be excluded from queries.
    ///
    /// This method is idempotent, and will not add the same component multiple times.
    ///
    /// # Warning
    ///
    /// This method should only be called before the app starts, as it will not affect queries
    /// initialized before it is called.
    ///
    /// As discussed in the [module docs](crate::entity_disabling), this can have performance implications,
    /// as well as create interoperability issues, and should be used with caution.
    pub fn register_disabling_component(&mut self, component_id: ComponentId) {
        if !self.disabling.contains(&component_id) {
            self.disabling.push(component_id);
        }
    }

    /// Get an iterator over all of the components which disable entities when present.
    pub fn disabling_ids(&self) -> impl Iterator<Item = ComponentId> + use<'_> {
        self.disabling.iter().copied()
    }

    /// Modifies the provided [`FilteredAccess`] to include the filters from this [`DefaultQueryFilters`].
    pub(super) fn modify_access(&self, component_access: &mut FilteredAccess<ComponentId>) {
        for component_id in self.disabling_ids() {
            if !component_access.contains(component_id) {
                component_access.and_without(component_id);
            }
        }
    }

    pub(super) fn is_dense(&self, components: &Components) -> bool {
        self.disabling_ids().all(|component_id| {
            components
                .get_info(component_id)
                .is_some_and(|info| info.storage_type() == StorageType::Table)
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        prelude::World,
        query::{Has, With},
    };
    use alloc::{vec, vec::Vec};

    #[test]
    fn filters_modify_access() {
        let mut filters = DefaultQueryFilters::empty();
        filters.register_disabling_component(ComponentId::new(1));

        // A component access with an unrelated component
        let mut component_access = FilteredAccess::<ComponentId>::default();
        component_access
            .access_mut()
            .add_component_read(ComponentId::new(2));

        let mut applied_access = component_access.clone();
        filters.modify_access(&mut applied_access);
        assert_eq!(0, applied_access.with_filters().count());
        assert_eq!(
            vec![ComponentId::new(1)],
            applied_access.without_filters().collect::<Vec<_>>()
        );

        // We add a with filter, now we expect to see both filters
        component_access.and_with(ComponentId::new(4));

        let mut applied_access = component_access.clone();
        filters.modify_access(&mut applied_access);
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
        filters.modify_access(&mut applied_access);
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
        filters.modify_access(&mut applied_access);
        assert_eq!(
            vec![ComponentId::new(4)],
            applied_access.with_filters().collect::<Vec<_>>()
        );
        assert_eq!(0, applied_access.without_filters().count());
    }

    #[derive(Component)]
    struct CustomDisabled;

    #[test]
    fn multiple_disabling_components() {
        let mut world = World::new();
        world.register_disabling_component::<CustomDisabled>();

        world.spawn_empty();
        world.spawn(Disabled);
        world.spawn(CustomDisabled);
        world.spawn((Disabled, CustomDisabled));

        let mut query = world.query::<()>();
        assert_eq!(1, query.iter(&world).count());

        let mut query = world.query_filtered::<(), With<Disabled>>();
        assert_eq!(1, query.iter(&world).count());

        let mut query = world.query::<Has<Disabled>>();
        assert_eq!(2, query.iter(&world).count());

        let mut query = world.query_filtered::<(), With<CustomDisabled>>();
        assert_eq!(1, query.iter(&world).count());

        let mut query = world.query::<Has<CustomDisabled>>();
        assert_eq!(2, query.iter(&world).count());

        let mut query = world.query_filtered::<(), (With<Disabled>, With<CustomDisabled>)>();
        assert_eq!(1, query.iter(&world).count());

        let mut query = world.query::<(Has<Disabled>, Has<CustomDisabled>)>();
        assert_eq!(4, query.iter(&world).count());
    }
}
