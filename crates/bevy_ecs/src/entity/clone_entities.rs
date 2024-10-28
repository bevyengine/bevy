use core::any::TypeId;

use bevy_utils::{HashSet, TypeIdMap};

use crate::{
    component::{Component, ComponentCloneHandler, ComponentCloneHandlers, ComponentId},
    entity::Entity,
    world::World,
};

/// A helper struct to clone an entity. Used internally by [`EntityCloneBuilder::clone_entity`] and custom clone handlers.
pub struct EntityCloner<'a> {
    source: Entity,
    target: Entity,
    allowed: bool,
    filter: &'a HashSet<ComponentId>,
    clone_handlers_overrides: &'a ComponentCloneHandlers,
}

impl<'a> EntityCloner<'a> {
    /// Clones and inserts components from the `source` entity into `target` entity using the stored configuration.
    pub fn clone_entity(&self, world: &mut World) {
        let components = world
            .get_entity(self.source)
            .unwrap()
            .archetype()
            .components()
            .collect::<Vec<_>>();

        for component in components {
            if !self.is_cloning_allowed(&component) {
                continue;
            }
            let handler = if self
                .clone_handlers_overrides
                .is_handler_registered(component)
            {
                self.clone_handlers_overrides.get_handler(component)
            } else {
                world
                    .components()
                    .get_component_clone_handlers()
                    .get_handler(component)
            };

            (handler)(world, component, self);
        }
    }

    fn is_cloning_allowed(&self, component: &ComponentId) -> bool {
        (self.allowed && self.filter.contains(component))
            || (!self.allowed && !self.filter.contains(component))
    }

    /// Returns the current source entity.
    pub fn get_source(&self) -> Entity {
        self.source
    }

    /// Returns the current target entity.
    pub fn get_target(&self) -> Entity {
        self.target
    }

    /// Reuse existing [`EntityCloner`] configuration with new source and target.
    pub fn with_source_and_target(&self, source: Entity, target: Entity) -> EntityCloner<'a> {
        EntityCloner {
            source,
            target,
            ..*self
        }
    }
}

/// Builder struct to clone an entity. Allows configuring which components to clone, as well as how to clone them.
/// After configuration is complete an entity can be cloned using [`Self::clone_entity`].
///
///```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::entity::EntityCloneBuilder;
///
/// #[derive(Component, Clone, PartialEq, Eq)]
/// struct A {
///     field: usize,
/// }
///
/// let mut world = World::default();
///
/// let component = A { field: 5 };
///
/// let entity = world.spawn(component.clone()).id();
/// let entity_clone = world.spawn_empty().id();
///
/// EntityCloneBuilder::default().clone_entity(&mut world, entity, entity_clone);
///
/// assert!(world.get::<A>(entity_clone).is_some_and(|c| *c == component));
///```
#[derive(Default)]
pub struct EntityCloneBuilder {
    ignored_components: HashSet<TypeId>,
    allowed_components: HashSet<TypeId>,
    clone_handlers_overrides: TypeIdMap<ComponentCloneHandler>,
}

impl EntityCloneBuilder {
    /// Finish configuring the builder and clone an entity.
    pub fn clone_entity(self, world: &mut World, source: Entity, target: Entity) {
        let EntityCloneBuilder {
            ignored_components,
            allowed_components,
            clone_handlers_overrides,
            ..
        } = self;

        let mut component_clone_handlers = ComponentCloneHandlers::default();
        for (k, v) in clone_handlers_overrides.into_iter() {
            if let Some(component_id) = world.components().get_id(k) {
                component_clone_handlers.set_component_handler(component_id, v);
            };
        }

        let mut allowed = false;
        let filter = if allowed_components.is_empty() {
            ignored_components
                .iter()
                .flat_map(|type_id| world.components().get_id(*type_id))
                .collect::<HashSet<_>>()
        } else {
            allowed = true;
            allowed_components
                .difference(&ignored_components)
                .flat_map(|type_id| world.components().get_id(*type_id))
                .collect::<HashSet<_>>()
        };

        EntityCloner {
            source,
            target,
            allowed,
            filter: &filter,
            clone_handlers_overrides: &component_clone_handlers,
        }
        .clone_entity(world);
    }

    /// Add a component to the list of allowed components to clone.
    /// Calling this function automatically disallows all other components, only explicitly allowed ones will be cloned.
    pub fn allow<T: Component>(&mut self) -> &mut Self {
        self.allowed_components.insert(TypeId::of::<T>());
        self
    }

    /// Disallow a component from being cloned.
    /// If [`Self::allow`] was called beforehand, this will remove the component from allowed list.
    pub fn deny<T: Component>(&mut self) -> &mut Self {
        self.ignored_components.insert(TypeId::of::<T>());
        self
    }

    /// Overrides the [`ComponentCloneHandler`] for the specific component for this builder.
    /// This handler will be used to clone component instead of the global one defined by [`ComponentCloneHandlers`]
    pub fn override_component_clone_handler<T: Component>(
        &mut self,
        handler: ComponentCloneHandler,
    ) -> &mut Self {
        self.clone_handlers_overrides
            .insert(TypeId::of::<T>(), handler);
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_ecs, component::Component, entity::EntityCloneBuilder, world::World};

    #[cfg(feature = "bevy_reflect")]
    #[test]
    fn clone_entity_using_reflect() {
        use crate::reflect::{AppTypeRegistry, ReflectComponent};
        use bevy_reflect::Reflect;

        #[derive(Component, Reflect, Clone, PartialEq, Eq)]
        #[reflect(Component)]
        struct A {
            field: usize,
        }

        let mut world = World::default();
        world.init_resource::<AppTypeRegistry>();
        let registry = world.get_resource::<AppTypeRegistry>().unwrap();
        registry.write().register::<A>();

        let component = A { field: 5 };

        let e = world.spawn(component.clone()).id();
        let e_clone = world.spawn_empty().id();

        EntityCloneBuilder::default().clone_entity(&mut world, e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
    }

    #[test]
    fn clone_entity_using_clone() {
        #[derive(Component, Clone, PartialEq, Eq)]
        struct A {
            field: usize,
        }

        let mut world = World::default();

        let component = A { field: 5 };

        let e = world.spawn(component.clone()).id();
        let e_clone = world.spawn_empty().id();

        EntityCloneBuilder::default().clone_entity(&mut world, e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
    }

    #[cfg(feature = "bevy_reflect")]
    #[test]
    fn clone_entity_specialization() {
        use crate::reflect::{AppTypeRegistry, ReflectComponent};
        use bevy_reflect::Reflect;

        #[derive(Component, Reflect, PartialEq, Eq)]
        #[reflect(Component)]
        struct A {
            field: usize,
        }

        impl Clone for A {
            fn clone(&self) -> Self {
                Self { field: 10 }
            }
        }

        let mut world = World::default();
        world.init_resource::<AppTypeRegistry>();
        let registry = world.get_resource::<AppTypeRegistry>().unwrap();
        registry.write().register::<A>();

        let component = A { field: 5 };

        let e = world.spawn(component.clone()).id();
        let e_clone = world.spawn_empty().id();

        EntityCloneBuilder::default().clone_entity(&mut world, e, e_clone);

        assert!(world
            .get::<A>(e_clone)
            .is_some_and(|comp| *comp == A { field: 10 }));
    }

    #[test]
    fn clone_entity_with_allow_filter() {
        #[derive(Component, Clone, PartialEq, Eq)]
        struct A {
            field: usize,
        }

        #[derive(Component, Clone)]
        struct B;

        let mut world = World::default();

        let component = A { field: 5 };

        let e = world.spawn((component.clone(), B)).id();
        let e_clone = world.spawn_empty().id();

        let mut builder = EntityCloneBuilder::default();
        builder.allow::<A>();
        builder.clone_entity(&mut world, e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        assert!(world.get::<B>(e_clone).is_none());
    }

    #[test]
    fn clone_entity_with_deny_filter() {
        #[derive(Component, Clone, PartialEq, Eq)]
        struct A {
            field: usize,
        }

        #[derive(Component, Clone)]
        struct B;

        #[derive(Component, Clone)]
        struct C;

        let mut world = World::default();

        let component = A { field: 5 };

        let e = world.spawn((component.clone(), B, C)).id();
        let e_clone = world.spawn_empty().id();

        let mut builder = EntityCloneBuilder::default();
        builder.deny::<B>();
        builder.clone_entity(&mut world, e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        assert!(world.get::<B>(e_clone).is_none());
        assert!(world.get::<C>(e_clone).is_some());
    }

    #[test]
    fn clone_entity_with_override_allow_filter() {
        #[derive(Component, Clone, PartialEq, Eq)]
        struct A {
            field: usize,
        }

        #[derive(Component, Clone)]
        struct B;

        #[derive(Component, Clone)]
        struct C;

        let mut world = World::default();

        let component = A { field: 5 };

        let e = world.spawn((component.clone(), B, C)).id();
        let e_clone = world.spawn_empty().id();

        let mut builder = EntityCloneBuilder::default();
        builder.allow::<A>();
        builder.allow::<B>();
        builder.allow::<C>();
        builder.deny::<B>();
        builder.clone_entity(&mut world, e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        assert!(world.get::<B>(e_clone).is_none());
        assert!(world.get::<C>(e_clone).is_some());
    }
}
