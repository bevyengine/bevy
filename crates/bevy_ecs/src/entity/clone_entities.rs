use alloc::{borrow::ToOwned, vec::Vec};
use bevy_ptr::{Ptr, PtrMut};
use bumpalo::Bump;
use core::{any::TypeId, ptr::NonNull};

use bevy_utils::{HashMap, HashSet};

#[cfg(feature = "bevy_reflect")]
use alloc::boxed::Box;

#[cfg(feature = "portable-atomic")]
use portable_atomic_util::Arc;

#[cfg(not(feature = "portable-atomic"))]
use alloc::sync::Arc;

use crate::{
    bundle::Bundle,
    component::{Component, ComponentCloneHandler, ComponentId, ComponentInfo, Components},
    entity::Entity,
    query::DebugCheckedUnwrap,
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
    component_info: &'a ComponentInfo,
    entity_cloner: &'a EntityCloner,
    #[cfg(feature = "bevy_reflect")]
    type_registry: Option<&'a crate::reflect::AppTypeRegistry>,
    #[cfg(not(feature = "bevy_reflect"))]
    #[expect(dead_code)]
    type_registry: Option<()>,
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
        #[cfg(not(feature = "bevy_reflect"))] type_registry: Option<()>,
    ) -> Self {
        Self {
            component_id,
            source_component_ptr,
            target_components_ptrs,
            target_component_written: false,
            target_components_buffer,
            components,
            component_info: components.get_info_unchecked(component_id),
            entity_cloner,
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

    /// Returns the [`ComponentId`] of the component being cloned.
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    /// Returns the [`ComponentInfo`] of the component being cloned.
    pub fn component_info(&self) -> &ComponentInfo {
        self.component_info
    }

    /// Returns a reference to the component on the source entity.
    ///
    /// Will return `None` if `ComponentId` of requested component does not match `ComponentId` of source component
    pub fn read_source_component<T: Component>(&self) -> Option<&T> {
        if self
            .component_info
            .type_id()
            .is_some_and(|id| id == TypeId::of::<T>())
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
    /// - Registered [`ReflectFromPtr`](bevy_reflect::ReflectFromPtr)'s [`TypeId`] does not match component's [`TypeId`]
    #[cfg(feature = "bevy_reflect")]
    pub fn read_source_component_reflect(&self) -> Option<&dyn bevy_reflect::Reflect> {
        let registry = self.type_registry?.read();
        let type_id = self.component_info.type_id()?;
        let reflect_from_ptr = registry.get_type_data::<bevy_reflect::ReflectFromPtr>(type_id)?;
        if reflect_from_ptr.type_id() != type_id {
            return None;
        }
        // SAFETY: `source_component_ptr` stores data represented by `component_id`, which we used to get `ReflectFromPtr`.
        unsafe { Some(reflect_from_ptr.as_reflect(self.source_component_ptr)) }
    }

    /// Writes component data to target entity.
    ///
    /// # Panics
    /// This will panic if:
    /// - Component has already been written once.
    /// - Component being written is not registered in the world.
    /// - `ComponentId` of component being written does not match expected `ComponentId`.
    pub fn write_target_component<T: Component>(&mut self, component: T) {
        let short_name = disqualified::ShortName::of::<T>();
        if self.target_component_written {
            panic!("Trying to write component '{short_name}' multiple times")
        }
        if self
            .component_info
            .type_id()
            .is_none_or(|id| id != TypeId::of::<T>())
        {
            panic!("TypeId of component '{short_name}' does not match source component TypeId")
        };
        let component_ref = self.target_components_buffer.alloc(component);
        self.target_components_ptrs
            .push(PtrMut::from(component_ref));
        self.target_component_written = true;
    }

    /// Writes component data to target entity by providing a pointer to source component data and a pointer to uninitialized target component data.
    ///
    /// This method allows caller to provide a function (`clone_fn`) to clone component using untyped pointers.
    /// First argument to `clone_fn` points to source component data ([`Ptr`]), second argument points to uninitialized buffer ([`NonNull`]) allocated with layout
    /// described by [`ComponentInfo`] stored in this [`ComponentCloneCtx`]. If cloning is successful and uninitialized buffer contains a valid clone of
    /// source component, `clone_fn` should return `true`, otherwise it should return `false`.
    ///
    /// # Safety
    /// Caller must ensure that if `clone_fn` is called and returns `true`, the second argument ([`NonNull`] pointer) points to a valid component data
    /// described by [`ComponentInfo`] stored in this [`ComponentCloneCtx`].
    /// # Panics
    /// This will panic if component has already been written once.
    pub unsafe fn write_target_component_ptr(
        &mut self,
        clone_fn: impl FnOnce(Ptr, NonNull<u8>) -> bool,
    ) {
        if self.target_component_written {
            panic!("Trying to write component multiple times")
        }
        let layout = self.component_info.layout();
        let target_component_data_ptr = self.target_components_buffer.alloc_layout(layout);

        if clone_fn(self.source_component_ptr, target_component_data_ptr) {
            self.target_components_ptrs
                .push(PtrMut::new(target_component_data_ptr));
            self.target_component_written = true;
        }
    }

    /// Writes component data to target entity.
    ///
    /// # Panics
    /// This will panic if:
    /// - World does not have [`AppTypeRegistry`](`crate::reflect::AppTypeRegistry`).
    /// - Component does not implement [`ReflectFromPtr`](bevy_reflect::ReflectFromPtr).
    /// - Source component does not have [`TypeId`].
    /// - Passed component's [`TypeId`] does not match source component [`TypeId`].
    /// - Component has already been written once.
    #[cfg(feature = "bevy_reflect")]
    pub fn write_target_component_reflect(&mut self, component: Box<dyn bevy_reflect::Reflect>) {
        if self.target_component_written {
            panic!("Trying to write component multiple times")
        }
        let source_type_id = self
            .component_info
            .type_id()
            .expect("Source component must have TypeId");
        let component_type_id = component.type_id();
        if source_type_id != component_type_id {
            panic!("Passed component TypeId does not match source component TypeId")
        }
        let component_layout = self.component_info.layout();

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
    move_components: bool,
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
            let app_registry = Option::<()>::None;

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

            // SAFETY:
            // - There are no other mutable references to source entity.
            // - `component` is from `source_entity`'s archetype
            let source_component_ptr =
                unsafe { source_entity.get_by_id(component).debug_checked_unwrap() };

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

        if self.move_components {
            world.entity_mut(self.source).remove_by_ids(&component_ids);
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
            filter_allows_components,
            filter: Arc::new(filter),
            clone_handlers_overrides: Arc::new(clone_handlers_overrides),
            move_components,
        }
        .clone_entity(world);
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
    use super::ComponentCloneCtx;
    use crate::{
        self as bevy_ecs,
        component::{Component, ComponentCloneHandler, ComponentDescriptor, StorageType},
        entity::EntityCloneBuilder,
        world::{DeferredWorld, World},
    };
    use bevy_ecs_macros::require;
    use bevy_ptr::OwningPtr;
    use core::alloc::Layout;

    #[cfg(feature = "bevy_reflect")]
    mod reflect {
        use super::*;
        use crate::reflect::{AppTypeRegistry, ReflectComponent, ReflectFromWorld};
        use bevy_reflect::{std_traits::ReflectDefault, FromType, Reflect, ReflectFromPtr};

        #[test]
        fn clone_entity_using_reflect() {
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

        // TODO: remove this when https://github.com/bevyengine/bevy/pull/13432 lands
        #[test]
        fn clone_entity_using_reflect_all_paths() {
            // `ReflectDefault`-based fast path
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            #[reflect(Default)]
            #[reflect(from_reflect = false)]
            struct A {
                field: usize,
                field2: Vec<usize>,
            }

            // `ReflectFromReflect`-based fast path
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            struct B {
                field: usize,
                field2: Vec<usize>,
            }

            // `ReflectFromWorld`-based fast path
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            #[reflect(FromWorld)]
            #[reflect(from_reflect = false)]
            struct C {
                field: usize,
                field2: Vec<usize>,
            }

            let mut world = World::default();
            world.init_resource::<AppTypeRegistry>();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            registry.write().register::<(A, B, C)>();

            let a_id = world.register_component::<A>();
            let b_id = world.register_component::<B>();
            let c_id = world.register_component::<C>();
            let handlers = world.get_component_clone_handlers_mut();
            handlers.set_component_handler(a_id, ComponentCloneHandler::reflect_handler());
            handlers.set_component_handler(b_id, ComponentCloneHandler::reflect_handler());
            handlers.set_component_handler(c_id, ComponentCloneHandler::reflect_handler());

            let component_a = A {
                field: 5,
                field2: vec![1, 2, 3, 4, 5],
            };
            let component_b = B {
                field: 6,
                field2: vec![1, 2, 3, 4, 5],
            };
            let component_c = C {
                field: 7,
                field2: vec![1, 2, 3, 4, 5],
            };

            let e = world.spawn((component_a, component_b, component_c)).id();
            let e_clone = world.spawn_empty().id();

            EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);

            assert_eq!(world.get::<A>(e_clone), Some(world.get::<A>(e).unwrap()));
            assert_eq!(world.get::<B>(e_clone), Some(world.get::<B>(e).unwrap()));
            assert_eq!(world.get::<C>(e_clone), Some(world.get::<C>(e).unwrap()));
        }

        #[test]
        fn read_source_component_reflect_should_return_none_on_invalid_reflect_from_ptr() {
            #[derive(Component, Reflect)]
            struct A;

            #[derive(Component, Reflect)]
            struct B;

            fn test_handler(_world: &mut DeferredWorld, ctx: &mut ComponentCloneCtx) {
                assert!(ctx.read_source_component_reflect().is_none());
            }

            let mut world = World::default();
            world.init_resource::<AppTypeRegistry>();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            {
                let mut registry = registry.write();
                registry.register::<A>();
                registry
                    .get_mut(core::any::TypeId::of::<A>())
                    .unwrap()
                    .insert(<ReflectFromPtr as FromType<B>>::from_type());
            }

            let a_id = world.register_component::<A>();
            let handlers = world.get_component_clone_handlers_mut();
            handlers
                .set_component_handler(a_id, ComponentCloneHandler::custom_handler(test_handler));

            let e = world.spawn(A).id();
            let e_clone = world.spawn_empty().id();

            EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);
        }

        #[test]
        fn clone_entity_specialization() {
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
        fn clone_entity_using_reflect_should_skip_without_panic() {
            // Not reflected
            #[derive(Component, PartialEq, Eq, Default, Debug)]
            struct A;

            // No valid type data
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            #[reflect(Component)]
            #[reflect(from_reflect = false)]
            struct B;

            let mut world = World::default();
            let a_id = world.register_component::<A>();
            let b_id = world.register_component::<B>();
            let handlers = world.get_component_clone_handlers_mut();
            handlers.set_component_handler(a_id, ComponentCloneHandler::reflect_handler());
            handlers.set_component_handler(b_id, ComponentCloneHandler::reflect_handler());

            // No AppTypeRegistry
            let e = world.spawn((A, B)).id();
            let e_clone = world.spawn_empty().id();
            EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);
            assert_eq!(world.get::<A>(e_clone), None);
            assert_eq!(world.get::<B>(e_clone), None);

            // With AppTypeRegistry
            world.init_resource::<AppTypeRegistry>();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            registry.write().register::<B>();

            let e = world.spawn((A, B)).id();
            let e_clone = world.spawn_empty().id();
            EntityCloneBuilder::new(&mut world).clone_entity(e, e_clone);
            assert_eq!(world.get::<A>(e_clone), None);
            assert_eq!(world.get::<B>(e_clone), None);
        }
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

    #[test]
    fn clone_entity_with_dynamic_components() {
        const COMPONENT_SIZE: usize = 10;
        fn test_handler(_world: &mut DeferredWorld, ctx: &mut ComponentCloneCtx) {
            // SAFETY: this handler is only going to be used with a component represented by [u8; COMPONENT_SIZE]
            unsafe {
                ctx.write_target_component_ptr(move |source_ptr, target_ptr| {
                    core::ptr::copy_nonoverlapping(
                        source_ptr.as_ptr(),
                        target_ptr.as_ptr(),
                        COMPONENT_SIZE,
                    );
                    true
                });
            }
        }

        let mut world = World::default();

        let layout = Layout::array::<u8>(COMPONENT_SIZE).unwrap();
        // SAFETY:
        // - No drop command is required
        // - The component will store [u8; COMPONENT_SIZE], which is Send + Sync
        let descriptor = unsafe {
            ComponentDescriptor::new_with_layout(
                "DynamicComp",
                StorageType::Table,
                layout,
                None,
                true,
            )
        };
        let component_id = world.register_component_with_descriptor(descriptor);

        let handlers = world.get_component_clone_handlers_mut();
        handlers.set_component_handler(
            component_id,
            ComponentCloneHandler::custom_handler(test_handler),
        );

        let mut entity = world.spawn_empty();
        let data = [5u8; COMPONENT_SIZE];

        // SAFETY:
        // - ptr points to data represented by component_id ([u8; COMPONENT_SIZE])
        // - component_id is from the same world as entity
        OwningPtr::make(data, |ptr| unsafe {
            entity.insert_by_id(component_id, ptr);
        });
        let entity = entity.id();

        let entity_clone = world.spawn_empty().id();
        let builder = EntityCloneBuilder::new(&mut world);
        builder.clone_entity(entity, entity_clone);

        let ptr = world.get_by_id(entity, component_id).unwrap();
        let clone_ptr = world.get_by_id(entity_clone, component_id).unwrap();
        // SAFETY: ptr and clone_ptr store component represented by [u8; COMPONENT_SIZE]
        unsafe {
            assert_eq!(
                core::slice::from_raw_parts(ptr.as_ptr(), COMPONENT_SIZE),
                core::slice::from_raw_parts(clone_ptr.as_ptr(), COMPONENT_SIZE),
            );
        }
    }
}
