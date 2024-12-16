use alloc::sync::Arc;
use core::any::TypeId;

use bevy_utils::{HashMap, HashSet};

use crate::{
    bundle::Bundle,
    component::{component_clone_ignore, Component, ComponentCloneHandler, ComponentId},
    entity::Entity,
    world::World,
};

/// A helper struct to clone an entity. Used internally by [`EntityCloneBuilder::clone_entity`] and custom clone handlers.
pub struct EntityCloner {
    source: Entity,
    target: Entity,
    component_id: Option<ComponentId>,
    filter_allows_components: bool,
    filter: Arc<HashSet<ComponentId>>,
    clone_handlers_overrides: Arc<HashMap<ComponentId, ComponentCloneHandler>>,
    move_components: bool,
}

impl EntityCloner {
    /// Clones and inserts components from the `source` entity into `target` entity using the stored configuration.
    pub fn clone_entity(&mut self, world: &mut World) {
        let source_entity = world
            .get_entity(self.source)
            .expect("Source entity must exist");
        let archetype = source_entity.archetype();

        let mut components = Vec::with_capacity(archetype.component_count());
        components.extend(
            archetype
                .components()
                .filter(|id| self.is_cloning_allowed(id)),
        );

        for component in &components {
            let global_handlers = world.components().get_component_clone_handlers();
            let handler = match self.clone_handlers_overrides.get(component) {
                None => global_handlers.get_handler(*component),
                Some(ComponentCloneHandler::Default) => global_handlers.get_default_handler(),
                Some(ComponentCloneHandler::Ignore) => component_clone_ignore,
                Some(ComponentCloneHandler::Custom(handler)) => *handler,
            };
            self.component_id = Some(*component);
            (handler)(&mut world.into(), self);
        }

        if self.move_components {
            world.entity_mut(self.source).remove_by_ids(&components);
        }
    }

    fn is_cloning_allowed(&self, component: &ComponentId) -> bool {
        (self.filter_allows_components && self.filter.contains(component))
            || (!self.filter_allows_components && !self.filter.contains(component))
    }

    /// Returns the current source entity.
    pub fn source(&self) -> Entity {
        self.source
    }

    /// Returns the current target entity.
    pub fn target(&self) -> Entity {
        self.target
    }

    /// Returns the [`ComponentId`] of currently cloned component.
    pub fn component_id(&self) -> ComponentId {
        self.component_id
            .expect("ComponentId must be set in clone_entity")
    }

    /// Reuse existing [`EntityCloner`] configuration with new source and target.
    pub fn with_source_and_target(&self, source: Entity, target: Entity) -> EntityCloner {
        EntityCloner {
            source,
            target,
            filter: self.filter.clone(),
            clone_handlers_overrides: self.clone_handlers_overrides.clone(),
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
/// EntityCloneBuilder::new(&mut world).clone_entity(entity, entity_clone);
///
/// assert!(world.get::<A>(entity_clone).is_some_and(|c| *c == component));
///```
///
/// # Default cloning strategy
/// By default, all types that derive [`Component`] and implement either [`Clone`] or `Reflect` (with `ReflectComponent`) will be cloned
/// (with `Clone`-based implementation preferred in case component implements both).
///
/// It should be noted that if `Component` is implemented manually or if `Clone` implementation is conditional
/// (like when deriving `Clone` for a type with a generic parameter without `Clone` bound),
/// the component will be cloned using the [default cloning strategy](crate::component::ComponentCloneHandlers::get_default_handler).
/// To use `Clone`-based handler ([`component_clone_via_clone`](crate::component::component_clone_via_clone)) in this case it should be set manually using one
/// of the methods mentioned in the [Handlers](#handlers) section
///
/// Here's an example of how to do it using [`get_component_clone_handler`](Component::get_component_clone_handler):
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::component::{StorageType, component_clone_via_clone, ComponentCloneHandler, Mutable};
/// #[derive(Clone)]
/// struct SomeComponent;
///
/// impl Component for SomeComponent {
///     const STORAGE_TYPE: StorageType = StorageType::Table;
///     type Mutability = Mutable;
///     fn get_component_clone_handler() -> ComponentCloneHandler {
///         ComponentCloneHandler::Custom(component_clone_via_clone::<Self>)
///     }
/// }
/// ```
///
/// # Handlers
/// `EntityCloneBuilder` clones entities by cloning components using [`handlers`](ComponentCloneHandler), and there are multiple layers
/// to decide which handler to use for which component. The overall hierarchy looks like this (priority from most to least):
/// 1. local overrides using [`override_component_clone_handler`](Self::override_component_clone_handler)
/// 2. global overrides using [`set_component_handler`](crate::component::ComponentCloneHandlers::set_component_handler)
/// 3. component-defined handler using [`get_component_clone_handler`](Component::get_component_clone_handler)
/// 4. default handler override using [`set_default_handler`](crate::component::ComponentCloneHandlers::set_default_handler)
/// 5. reflect-based or noop default clone handler depending on if `bevy_reflect` feature is enabled or not.
#[derive(Debug)]
pub struct EntityCloneBuilder<'w> {
    world: &'w mut World,
    filter_allows_components: bool,
    filter: HashSet<ComponentId>,
    clone_handlers_overrides: HashMap<ComponentId, ComponentCloneHandler>,
    attach_required_components: bool,
    move_components: bool,
}

impl<'w> EntityCloneBuilder<'w> {
    /// Creates a new [`EntityCloneBuilder`] for world.
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world,
            filter_allows_components: false,
            filter: Default::default(),
            clone_handlers_overrides: Default::default(),
            attach_required_components: true,
            move_components: false,
        }
    }

    /// Finishes configuring the builder and clones `source` entity to `target`.
    pub fn clone_entity(self, source: Entity, target: Entity) {
        let EntityCloneBuilder {
            world,
            filter_allows_components,
            filter,
            clone_handlers_overrides,
            move_components,
            ..
        } = self;

        EntityCloner {
            source,
            target,
            component_id: None,
            filter_allows_components,
            filter: Arc::new(filter),
            clone_handlers_overrides: Arc::new(clone_handlers_overrides),
            move_components,
        }
        .clone_entity(world);

        world.flush_commands();
    }

    /// By default, any components allowed/denied through the filter will automatically
    /// allow/deny all of their required components.
    ///
    /// This method allows for a scoped mode where any changes to the filter
    /// will not involve required components.
    pub fn without_required_components(
        &mut self,
        builder: impl FnOnce(&mut EntityCloneBuilder) + Send + Sync + 'static,
    ) -> &mut Self {
        self.attach_required_components = false;
        builder(self);
        self.attach_required_components = true;
        self
    }

    /// Sets whether the cloner should remove any components that were cloned,
    /// effectively moving them from the source entity to the target.
    ///
    /// This is disabled by default.
    ///
    /// The setting only applies to components that are allowed through the filter
    /// at the time [`EntityCloneBuilder::clone_entity`] is called.
    pub fn move_components(&mut self, enable: bool) -> &mut Self {
        self.move_components = enable;
        self
    }

    /// Adds all components of the bundle to the list of components to clone.
    ///
    /// Note that all components are allowed by default, to clone only explicitly allowed components make sure to call
    /// [`deny_all`](`Self::deny_all`) before calling any of the `allow` methods.
    pub fn allow<T: Bundle>(&mut self) -> &mut Self {
        let bundle = self.world.register_bundle::<T>();
        let ids = bundle.explicit_components().to_owned();
        for id in ids {
            self.filter_allow(id);
        }
        self
    }

    /// Extends the list of components to clone.
    ///
    /// Note that all components are allowed by default, to clone only explicitly allowed components make sure to call
    /// [`deny_all`](`Self::deny_all`) before calling any of the `allow` methods.
    pub fn allow_by_ids(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        for id in ids {
            self.filter_allow(id);
        }
        self
    }

    /// Extends the list of components to clone using [`TypeId`]s.
    ///
    /// Note that all components are allowed by default, to clone only explicitly allowed components make sure to call
    /// [`deny_all`](`Self::deny_all`) before calling any of the `allow` methods.
    pub fn allow_by_type_ids(&mut self, ids: impl IntoIterator<Item = TypeId>) -> &mut Self {
        for type_id in ids {
            if let Some(id) = self.world.components().get_id(type_id) {
                self.filter_allow(id);
            }
        }
        self
    }

    /// Resets the filter to allow all components to be cloned.
    pub fn allow_all(&mut self) -> &mut Self {
        self.filter_allows_components = false;
        self.filter.clear();
        self
    }

    /// Disallows all components of the bundle from being cloned.
    pub fn deny<T: Bundle>(&mut self) -> &mut Self {
        let bundle = self.world.register_bundle::<T>();
        let ids = bundle.explicit_components().to_owned();
        for id in ids {
            self.filter_deny(id);
        }
        self
    }

    /// Extends the list of components that shouldn't be cloned.
    pub fn deny_by_ids(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        for id in ids {
            self.filter_deny(id);
        }
        self
    }

    /// Extends the list of components that shouldn't be cloned by type ids.
    pub fn deny_by_type_ids(&mut self, ids: impl IntoIterator<Item = TypeId>) -> &mut Self {
        for type_id in ids {
            if let Some(id) = self.world.components().get_id(type_id) {
                self.filter_deny(id);
            }
        }
        self
    }

    /// Sets the filter to deny all components.
    pub fn deny_all(&mut self) -> &mut Self {
        self.filter_allows_components = true;
        self.filter.clear();
        self
    }

    /// Overrides the [`ComponentCloneHandler`] for a component in this builder.
    /// This handler will be used to clone the component instead of the global one defined by [`ComponentCloneHandlers`](crate::component::ComponentCloneHandlers)
    ///
    /// See [Handlers section of `EntityCloneBuilder`](EntityCloneBuilder#handlers) to understand how this affects handler priority.
    pub fn override_component_clone_handler<T: Component>(
        &mut self,
        handler: ComponentCloneHandler,
    ) -> &mut Self {
        if let Some(id) = self.world.components().component_id::<T>() {
            self.clone_handlers_overrides.insert(id, handler);
        }
        self
    }

    /// Removes a previously set override of [`ComponentCloneHandler`] for a component in this builder.
    pub fn remove_component_clone_handler_override<T: Component>(&mut self) -> &mut Self {
        if let Some(id) = self.world.components().component_id::<T>() {
            self.clone_handlers_overrides.remove(&id);
        }
        self
    }

    /// Helper function that allows a component through the filter.
    fn filter_allow(&mut self, id: ComponentId) {
        if self.filter_allows_components {
            self.filter.insert(id);
        } else {
            self.filter.remove(&id);
        }
        if self.attach_required_components {
            if let Some(info) = self.world.components().get_info(id) {
                for required_id in info.required_components().iter_ids() {
                    if self.filter_allows_components {
                        self.filter.insert(required_id);
                    } else {
                        self.filter.remove(&required_id);
                    }
                }
            }
        }
    }

    /// Helper function that disallows a component through the filter.
    fn filter_deny(&mut self, id: ComponentId) {
        if self.filter_allows_components {
            self.filter.remove(&id);
        } else {
            self.filter.insert(id);
        }
        if self.attach_required_components {
            if let Some(info) = self.world.components().get_info(id) {
                for required_id in info.required_components().iter_ids() {
                    if self.filter_allows_components {
                        self.filter.remove(&required_id);
                    } else {
                        self.filter.insert(required_id);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_ecs, component::Component, entity::EntityCloneBuilder, world::World};
    use bevy_ecs_macros::require;

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

        EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);

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

        EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);

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

        EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);

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

        let mut builder = EntityCloneBuilder::new(&mut world);
        builder.deny_all();
        builder.allow::<A>();
        builder.clone_entity(e, e_clone);

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

        let mut builder = EntityCloneBuilder::new(&mut world);
        builder.deny::<B>();
        builder.clone_entity(e, e_clone);

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

        let mut builder = EntityCloneBuilder::new(&mut world);
        builder.deny_all();
        builder.allow::<A>();
        builder.allow::<B>();
        builder.allow::<C>();
        builder.deny::<B>();
        builder.clone_entity(e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        assert!(world.get::<B>(e_clone).is_none());
        assert!(world.get::<C>(e_clone).is_some());
    }

    #[test]
    fn clone_entity_with_override_bundle() {
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

        let mut builder = EntityCloneBuilder::new(&mut world);
        builder.deny_all();
        builder.allow::<(A, B, C)>();
        builder.deny::<(B, C)>();
        builder.clone_entity(e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        assert!(world.get::<B>(e_clone).is_none());
        assert!(world.get::<C>(e_clone).is_none());
    }

    #[test]
    fn clone_entity_with_required_components() {
        #[derive(Component, Clone, PartialEq, Debug)]
        #[require(B)]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(C(|| C(5)))]
        struct B;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct C(u32);

        let mut world = World::default();

        let e = world.spawn(A).id();
        let e_clone = world.spawn_empty().id();

        let mut builder = EntityCloneBuilder::new(&mut world);
        builder.deny_all();
        builder.without_required_components(|builder| {
            builder.allow::<B>();
        });
        builder.clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), None);
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B));
        assert_eq!(world.entity(e_clone).get::<C>(), Some(&C(5)));
    }
}
