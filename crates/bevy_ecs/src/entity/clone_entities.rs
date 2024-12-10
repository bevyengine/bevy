use alloc::sync::Arc;
use bevy_ptr::{Ptr, PtrMut};
use bumpalo::Bump;
use core::any::TypeId;

use bevy_utils::{HashMap, HashSet};

use crate::{
    bundle::Bundle,
    component::{Component, ComponentCloneHandler, ComponentId, Components},
    entity::Entity,
    world::World,
};

/// Context for component clone handlers.
///
/// Provides fast access to useful resources like [`AppTypeRegistry`](crate::reflect::AppTypeRegistry)
/// and allows component clone handler to get information about component being cloned.
pub struct ComponentCloneCtx<'a, 'b> {
    component_id: ComponentId,
    source_component_ptr: Ptr<'a>,
    target_component_written: bool,
    target_components_ptrs: &'a mut Vec<PtrMut<'b>>,
    target_components_buffer: &'b Bump,
    components: &'a Components,
    entity_cloner: &'a EntityCloner,
    #[cfg(feature = "bevy_reflect")]
    type_registry: Option<&'a crate::reflect::AppTypeRegistry>,
}

impl<'a, 'b> ComponentCloneCtx<'a, 'b> {
    /// Create a new instance of `ComponentCloneCtx` that can be passed to component clone handlers.
    ///
    /// # Safety
    /// Caller must ensure that:
    /// - `components` and `component_id` are from the same world.
    /// - `source_component_ptr` points to a valid component of type represented by `component_id`.
    unsafe fn new(
        component_id: ComponentId,
        source_component_ptr: Ptr<'a>,
        target_components_ptrs: &'a mut Vec<PtrMut<'b>>,
        target_components_buffer: &'b Bump,
        components: &'a Components,
        entity_cloner: &'a EntityCloner,
        #[cfg(feature = "bevy_reflect")] type_registry: Option<&'a crate::reflect::AppTypeRegistry>,
    ) -> Self {
        Self {
            component_id,
            source_component_ptr,
            target_components_ptrs,
            target_component_written: false,
            target_components_buffer,
            components,
            entity_cloner,
            #[cfg(feature = "bevy_reflect")]
            type_registry,
        }
    }

    /// Returns true if [`write_target_component`](`Self::write_target_component`) was called before.
    pub fn target_component_written(&self) -> bool {
        self.target_component_written
    }

    /// Returns the current source entity.
    pub fn source(&self) -> Entity {
        self.entity_cloner.source
    }

    /// Returns the current target entity.
    pub fn target(&self) -> Entity {
        self.entity_cloner.target
    }

    /// Returns the [`ComponentId`] of currently cloned component.
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    /// Returns a reference to the component on the source entity.
    ///
    /// Will return `None` if `ComponentId` of requested component does not match `ComponentId` of source component
    pub fn read_source_component<T: Component>(&self) -> Option<&T> {
        if self
            .components
            .component_id::<T>()
            .is_some_and(|id| id == self.component_id)
        {
            // SAFETY:
            // - Components and ComponentId are from the same world
            // - source_component_ptr holds valid data of the type referenced by ComponentId
            unsafe { Some(self.source_component_ptr.deref::<T>()) }
        } else {
            None
        }
    }

    /// Returns a reference to the component on the source entity as [`&dyn Reflect`](bevy_reflect::Reflect).
    ///
    /// Will return `None` if:
    /// - World does not have [`AppTypeRegistry`](`crate::reflect::AppTypeRegistry`).
    /// - Component does not implement [`ReflectFromPtr`](bevy_reflect::ReflectFromPtr).
    /// - Component is not registered.
    /// - Component does not have [`TypeId`]
    #[cfg(feature = "bevy_reflect")]
    pub fn read_source_component_reflect(&self) -> Option<&dyn bevy_reflect::Reflect> {
        let registry = self.type_registry?.read();
        let type_id = self.components.get_info(self.component_id)?.type_id()?;
        let reflect_from_ptr = registry.get_type_data::<bevy_reflect::ReflectFromPtr>(type_id)?;
        // SAFETY: `source_component_ptr` stores data represented by `component_id`, which we used to get `ReflectFromPtr`.
        unsafe { Some(reflect_from_ptr.as_reflect(self.source_component_ptr)) }
    }

    /// Writes component data to target entity.
    ///
    /// # Panics
    /// This will panic if:
    /// - `write_target_component` called more than once.
    /// - Component being written is not registered in the world.
    /// - `ComponentId` of component being written does not match expected `ComponentId`.
    pub fn write_target_component<T: Component>(&mut self, component: T) {
        let short_name = disqualified::ShortName::of::<T>();
        if self.target_component_written {
            panic!("Trying to write component '{short_name}' multiple times")
        }
        let Some(component_id) = self.components.component_id::<T>() else {
            panic!("Component '{short_name}' is not registered")
        };
        if component_id != self.component_id {
            panic!("Component '{short_name}' does not match ComponentId of this ComponentCloneCtx");
        }
        let component_ref = self.target_components_buffer.alloc(component);
        self.target_components_ptrs
            .push(PtrMut::from(component_ref));
        self.target_component_written = true;
    }

    /// Writes component data to target entity.
    ///
    /// # Panics
    /// This will panic if:
    /// - World does not have [`AppTypeRegistry`](`crate::reflect::AppTypeRegistry`).
    /// - Component does not implement [`ReflectFromPtr`](bevy_reflect::ReflectFromPtr).
    /// - Component is not registered.
    /// - Component does not have [`TypeId`]
    /// - Passed component's [`TypeId`] does not match source component [`TypeId`]
    #[cfg(feature = "bevy_reflect")]
    pub fn write_target_component_reflect(&mut self, component: Box<dyn bevy_reflect::Reflect>) {
        let source_type_id = self
            .components
            .get_info(self.component_id)
            .unwrap()
            .type_id()
            .unwrap();
        let component_type_id = component.reflect_type_info().type_id();
        if source_type_id != component_type_id {
            panic!("Passed component TypeId does not match source component TypeId")
        }
        let component_info = self.components.get_info(self.component_id).unwrap();
        let component_layout = component_info.layout();

        let component_data_ptr = Box::into_raw(component).cast::<u8>();
        let target_component_data_ptr =
            self.target_components_buffer.alloc_layout(component_layout);
        // SAFETY:
        // - target_component_data_ptr and component_data have the same data type.
        // - component_data_ptr has layout of component_layout
        unsafe {
            core::ptr::copy_nonoverlapping(
                component_data_ptr,
                target_component_data_ptr.as_ptr(),
                component_layout.size(),
            );
            self.target_components_ptrs
                .push(PtrMut::new(target_component_data_ptr));
            alloc::alloc::dealloc(component_data_ptr, component_layout);
        }

        self.target_component_written = true;
    }

    /// Return a reference to this context's `EntityCloner` instance.
    ///
    /// This can be used to issue clone commands using the same cloning configuration:
    /// ```
    /// # use bevy_ecs::world::{DeferredWorld, World};
    /// # use bevy_ecs::entity::ComponentCloneCtx;
    /// fn clone_handler(world: &mut DeferredWorld, ctx: &mut ComponentCloneCtx) {
    ///     let another_target = world.commands().spawn_empty().id();
    ///     let mut entity_cloner = ctx
    ///         .entity_cloner()
    ///         .with_source_and_target(ctx.source(), another_target);
    ///     world.commands().queue(move |world: &mut World| {
    ///         entity_cloner.clone_entity(world);
    ///     });
    /// }
    /// ```
    pub fn entity_cloner(&self) -> &EntityCloner {
        self.entity_cloner
    }

    /// Returns instance of [`Components`].
    pub fn components(&self) -> &Components {
        self.components
    }

    /// Returns [`AppTypeRegistry`](`crate::reflect::AppTypeRegistry`) if it exists in the world.
    ///
    /// NOTE: Prefer this method instead of manually reading the resource from the world.
    #[cfg(feature = "bevy_reflect")]
    pub fn type_registry(&self) -> Option<&crate::reflect::AppTypeRegistry> {
        self.type_registry
    }
}

/// A helper struct to clone an entity. Used internally by [`EntityCloneBuilder::clone_entity`].
pub struct EntityCloner {
    source: Entity,
    target: Entity,
    filter_allows_components: bool,
    filter: Arc<HashSet<ComponentId>>,
    clone_handlers_overrides: Arc<HashMap<ComponentId, ComponentCloneHandler>>,
}

impl EntityCloner {
    /// Clones and inserts components from the `source` entity into `target` entity using the stored configuration.
    pub fn clone_entity(&mut self, world: &mut World) {
        // SAFETY:
        // - `source_entity` is read-only.
        // - `type_registry` is read-only.
        // - `components` is read-only.
        // - `deferred_world` disallows structural ecs changes, which means all read-only resources above a not affected.
        let (type_registry, source_entity, components, mut deferred_world) = unsafe {
            let world = world.as_unsafe_world_cell();
            let source_entity = world
                .get_entity(self.source)
                .expect("Source entity must exist");

            #[cfg(feature = "bevy_reflect")]
            let app_registry = world.get_resource::<crate::reflect::AppTypeRegistry>();
            #[cfg(not(feature = "bevy_reflect"))]
            let app_registry = None;

            (
                app_registry,
                source_entity,
                world.components(),
                world.into_deferred(),
            )
        };
        let archetype = source_entity.archetype();

        let component_data = Bump::new();
        let mut component_ids: Vec<ComponentId> = Vec::with_capacity(archetype.component_count());
        let mut component_data_ptrs: Vec<PtrMut> = Vec::with_capacity(archetype.component_count());

        for component in archetype.components() {
            if !self.is_cloning_allowed(&component) {
                continue;
            }

            let global_handlers = components.get_component_clone_handlers();
            let handler = match self.clone_handlers_overrides.get(&component) {
                Some(handler) => handler
                    .get_handler()
                    .unwrap_or_else(|| global_handlers.get_default_handler()),
                None => global_handlers.get_handler(component),
            };

            // SAFETY: There are no other mutable references to source entity.
            let Some(source_component_ptr) = (unsafe { source_entity.get_by_id(component) }) else {
                continue;
            };

            // SAFETY:
            // - `components` and `component` are from the same world
            // - `source_component_ptr` is valid and points to the same type as represented by `component`
            let mut ctx = unsafe {
                ComponentCloneCtx::new(
                    component,
                    source_component_ptr,
                    &mut component_data_ptrs,
                    &component_data,
                    components,
                    self,
                    #[cfg(feature = "bevy_reflect")]
                    type_registry,
                )
            };

            (handler)(&mut deferred_world, &mut ctx);

            if ctx.target_component_written {
                component_ids.push(component);
            }
        }

        world.flush();

        if !world.entities.contains(self.target) {
            panic!("Target entity does not exist");
        }

        debug_assert_eq!(component_data_ptrs.len(), component_ids.len());

        // SAFETY:
        // - All `component_ids` are from the same world as `target` entity
        // - All `component_data_ptrs` are valid types represented by `component_ids`
        unsafe {
            world.entity_mut(self.target).insert_by_ids(
                &component_ids,
                component_data_ptrs.into_iter().map(|ptr| ptr.promote()),
            );
        }
    }

    fn is_cloning_allowed(&self, component: &ComponentId) -> bool {
        (self.filter_allows_components && self.filter.contains(component))
            || (!self.filter_allows_components && !self.filter.contains(component))
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
/// To use `Clone`-based handler ([`ComponentCloneHandler::clone_handler`]) in this case it should be set manually using one
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
///         ComponentCloneHandler::clone_handler::<Self>()
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
}

impl<'w> EntityCloneBuilder<'w> {
    /// Creates a new [`EntityCloneBuilder`] for world.
    pub fn new(world: &'w mut World) -> Self {
        Self {
            world,
            filter_allows_components: false,
            filter: Default::default(),
            clone_handlers_overrides: Default::default(),
        }
    }

    /// Finishes configuring the builder and clones `source` entity to `target`.
    pub fn clone_entity(self, source: Entity, target: Entity) {
        let EntityCloneBuilder {
            world,
            filter_allows_components,
            filter,
            clone_handlers_overrides,
            ..
        } = self;

        EntityCloner {
            source,
            target,
            filter_allows_components,
            filter: Arc::new(filter),
            clone_handlers_overrides: Arc::new(clone_handlers_overrides),
        }
        .clone_entity(world);
    }

    /// Adds all components of the bundle to the list of components to clone.
    ///
    /// Note that all components are allowed by default, to clone only explicitly allowed components make sure to call
    /// [`deny_all`](`Self::deny_all`) before calling any of the `allow` methods.
    pub fn allow<T: Bundle>(&mut self) -> &mut Self {
        if self.filter_allows_components {
            T::get_component_ids(self.world.components(), &mut |id| {
                if let Some(id) = id {
                    self.filter.insert(id);
                }
            });
        } else {
            T::get_component_ids(self.world.components(), &mut |id| {
                if let Some(id) = id {
                    self.filter.remove(&id);
                }
            });
        }
        self
    }

    /// Extends the list of components to clone.
    ///
    /// Note that all components are allowed by default, to clone only explicitly allowed components make sure to call
    /// [`deny_all`](`Self::deny_all`) before calling any of the `allow` methods.
    pub fn allow_by_ids(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        if self.filter_allows_components {
            self.filter.extend(ids);
        } else {
            ids.into_iter().for_each(|id| {
                self.filter.remove(&id);
            });
        }
        self
    }

    /// Extends the list of components to clone using [`TypeId`]s.
    ///
    /// Note that all components are allowed by default, to clone only explicitly allowed components make sure to call
    /// [`deny_all`](`Self::deny_all`) before calling any of the `allow` methods.
    pub fn allow_by_type_ids(&mut self, ids: impl IntoIterator<Item = TypeId>) -> &mut Self {
        let ids = ids
            .into_iter()
            .filter_map(|id| self.world.components().get_id(id));
        if self.filter_allows_components {
            self.filter.extend(ids);
        } else {
            ids.into_iter().for_each(|id| {
                self.filter.remove(&id);
            });
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
        if self.filter_allows_components {
            T::get_component_ids(self.world.components(), &mut |id| {
                if let Some(id) = id {
                    self.filter.remove(&id);
                }
            });
        } else {
            T::get_component_ids(self.world.components(), &mut |id| {
                if let Some(id) = id {
                    self.filter.insert(id);
                }
            });
        }
        self
    }

    /// Extends the list of components that shouldn't be cloned.
    pub fn deny_by_ids(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        if self.filter_allows_components {
            ids.into_iter().for_each(|id| {
                self.filter.remove(&id);
            });
        } else {
            self.filter.extend(ids);
        }
        self
    }

    /// Extends the list of components that shouldn't be cloned by type ids.
    pub fn deny_by_type_ids(&mut self, ids: impl IntoIterator<Item = TypeId>) -> &mut Self {
        let ids = ids
            .into_iter()
            .filter_map(|id| self.world.components().get_id(id));
        if self.filter_allows_components {
            ids.into_iter().for_each(|id| {
                self.filter.remove(&id);
            });
        } else {
            self.filter.extend(ids);
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
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs, component::Component, component::ComponentCloneHandler,
        entity::EntityCloneBuilder, world::World,
    };

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

        world.register_component::<A>();
        let id = world.component_id::<A>().unwrap();
        world
            .get_component_clone_handlers_mut()
            .set_component_handler(id, ComponentCloneHandler::reflect_handler());

        let component = A { field: 5 };

        let e = world.spawn(component.clone()).id();
        let e_clone = world.spawn_empty().id();

        EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
    }

    // TODO: remove this when 13432 lands
    #[cfg(feature = "bevy_reflect")]
    #[test]
    fn clone_entity_using_reflect_with_default() {
        use crate::reflect::{AppTypeRegistry, ReflectComponent};
        use bevy_reflect::{std_traits::ReflectDefault, Reflect};

        #[derive(Component, Reflect, Clone, PartialEq, Eq, Default)]
        #[reflect(Component, Default)]
        struct A {
            field: usize,
            field2: Vec<usize>,
        }

        let mut world = World::default();
        world.init_resource::<AppTypeRegistry>();
        let registry = world.get_resource::<AppTypeRegistry>().unwrap();
        registry.write().register::<A>();

        world.register_component::<A>();
        let id = world.component_id::<A>().unwrap();
        world
            .get_component_clone_handlers_mut()
            .set_component_handler(id, ComponentCloneHandler::reflect_handler());

        let component = A {
            field: 5,
            field2: vec![1, 2, 3, 4, 5],
        };

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
}
