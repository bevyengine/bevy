use alloc::{borrow::ToOwned, vec::Vec};
use bevy_platform_support::collections::{HashMap, HashSet};
use bevy_ptr::{Ptr, PtrMut};
use bumpalo::Bump;
use core::{any::TypeId, ptr::NonNull};

#[cfg(feature = "bevy_reflect")]
use alloc::boxed::Box;

use crate::component::{ComponentCloneBehavior, ComponentCloneFn};
use crate::entity::hash_map::EntityHashMap;
use crate::entity::EntityMapper;
use crate::system::Commands;
use crate::{
    bundle::Bundle,
    component::{Component, ComponentId, ComponentInfo, Components},
    entity::Entity,
    query::DebugCheckedUnwrap,
    world::World,
};
use alloc::collections::VecDeque;
use core::cell::RefCell;

/// Context for component clone handlers.
///
/// Provides fast access to useful resources like [`AppTypeRegistry`](crate::reflect::AppTypeRegistry)
/// and allows component clone handler to get information about component being cloned.
pub struct ComponentCloneCtx<'a, 'b> {
    component_id: ComponentId,
    source_component_ptr: Ptr<'a>,
    target_component_written: bool,
    bundle_scratch: &'a mut BundleScratch<'b>,
    bundle_scratch_allocator: &'b Bump,
    source: Entity,
    target: Entity,
    components: &'a Components,
    component_info: &'a ComponentInfo,
    entity_cloner: &'a mut EntityCloner,
    mapper: &'a mut dyn EntityMapper,
    #[cfg(feature = "bevy_reflect")]
    type_registry: Option<&'a crate::reflect::AppTypeRegistry>,
    #[cfg(not(feature = "bevy_reflect"))]
    #[expect(dead_code)]
    type_registry: Option<&'a ()>,
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
        source: Entity,
        target: Entity,
        source_component_ptr: Ptr<'a>,
        bundle_scratch_allocator: &'b Bump,
        bundle_scratch: &'a mut BundleScratch<'b>,
        components: &'a Components,
        entity_cloner: &'a mut EntityCloner,
        mapper: &'a mut dyn EntityMapper,
        #[cfg(feature = "bevy_reflect")] type_registry: Option<&'a crate::reflect::AppTypeRegistry>,
        #[cfg(not(feature = "bevy_reflect"))] type_registry: Option<&'a ()>,
    ) -> Self {
        Self {
            component_id,
            source,
            target,
            source_component_ptr,
            bundle_scratch,
            target_component_written: false,
            bundle_scratch_allocator,
            components,
            mapper,
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
        self.source
    }

    /// Returns the current target entity.
    pub fn target(&self) -> Entity {
        self.target
    }

    /// Returns the [`ComponentId`] of the component being cloned.
    pub fn component_id(&self) -> ComponentId {
        self.component_id
    }

    /// Returns the [`ComponentInfo`] of the component being cloned.
    pub fn component_info(&self) -> &ComponentInfo {
        self.component_info
    }

    /// Returns true if the [`EntityCloner`] is configured to recursively clone entities. When this is enabled,
    /// entities stored in a cloned entity's [`RelationshipTarget`](crate::relationship::RelationshipTarget) component with
    /// [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN) will also be cloned.
    #[inline]
    pub fn is_recursive(&self) -> bool {
        self.entity_cloner.is_recursive
    }

    /// Returns this context's [`EntityMapper`].
    pub fn entity_mapper(&mut self) -> &mut dyn EntityMapper {
        self.mapper
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
    pub fn write_target_component<C: Component>(&mut self, mut component: C) {
        C::visit_entities_mut(&mut component, |entity| {
            *entity = self.mapper.get_mapped(*entity);
        });
        let short_name = disqualified::ShortName::of::<C>();
        if self.target_component_written {
            panic!("Trying to write component '{short_name}' multiple times")
        }
        if self
            .component_info
            .type_id()
            .is_none_or(|id| id != TypeId::of::<C>())
        {
            panic!("TypeId of component '{short_name}' does not match source component TypeId")
        };
        // SAFETY: the TypeId of self.component_id has been checked to ensure it matches `C`
        unsafe {
            self.bundle_scratch
                .push(self.bundle_scratch_allocator, self.component_id, component);
        };
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
        let target_component_data_ptr = self.bundle_scratch_allocator.alloc_layout(layout);

        if clone_fn(self.source_component_ptr, target_component_data_ptr) {
            self.bundle_scratch
                .push_ptr(self.component_id, PtrMut::new(target_component_data_ptr));
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
            self.bundle_scratch_allocator.alloc_layout(component_layout);
        // SAFETY:
        // - target_component_data_ptr and component_data have the same data type.
        // - component_data_ptr has layout of component_layout
        unsafe {
            core::ptr::copy_nonoverlapping(
                component_data_ptr,
                target_component_data_ptr.as_ptr(),
                component_layout.size(),
            );
            self.bundle_scratch
                .push_ptr(self.component_id, PtrMut::new(target_component_data_ptr));
            alloc::alloc::dealloc(component_data_ptr, component_layout);
        }

        self.target_component_written = true;
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

    /// Queues the `entity` to be cloned by the current [`EntityCloner`]
    pub fn queue_entity_clone(&self, entity: Entity) {
        self.entity_cloner
            .clone_queue
            .borrow_mut()
            .push_back(entity);
    }
}

/// A configuration determining how to clone entities. This can be built using [`EntityCloner::build`], which
/// returns an [`EntityClonerBuilder`].
///
/// After configuration is complete an entity can be cloned using [`Self::clone_entity`].
///
///```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::entity::EntityCloner;
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
/// EntityCloner::build(&mut world).clone_entity(entity, entity_clone);
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
/// the component will be cloned using the [default cloning strategy](crate::component::ComponentCloneBehavior::global_default_fn).
/// To use `Clone`-based handler ([`ComponentCloneBehavior::clone`]) in this case it should be set manually using one
/// of the methods mentioned in the [Clone Behaviors](#Clone-Behaviors) section
///
/// Here's an example of how to do it using [`clone_behavior`](Component::clone_behavior):
/// ```
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::component::{StorageType, ComponentCloneBehavior, Mutable};
/// #[derive(Clone)]
/// struct SomeComponent;
///
/// impl Component for SomeComponent {
///     const STORAGE_TYPE: StorageType = StorageType::Table;
///     type Mutability = Mutable;
///     fn clone_behavior() -> ComponentCloneBehavior {
///         ComponentCloneBehavior::clone::<Self>()
///     }
/// }
/// ```
///
/// # Clone Behaviors
/// [`EntityCloner`] clones entities by cloning components using [`ComponentCloneBehavior`], and there are multiple layers
/// to decide which handler to use for which component. The overall hierarchy looks like this (priority from most to least):
/// 1. local overrides using [`EntityClonerBuilder::override_clone_behavior`]
/// 2. component-defined handler using [`Component::clone_behavior`]
/// 3. default handler override using [`EntityClonerBuilder::with_default_clone_fn`].
/// 4. reflect-based or noop default clone handler depending on if `bevy_reflect` feature is enabled or not.
#[derive(Debug)]
pub struct EntityCloner {
    filter_allows_components: bool,
    filter: HashSet<ComponentId>,
    clone_behavior_overrides: HashMap<ComponentId, ComponentCloneBehavior>,
    move_components: bool,
    is_recursive: bool,
    default_clone_fn: ComponentCloneFn,
    clone_queue: RefCell<VecDeque<Entity>>,
}

impl Default for EntityCloner {
    fn default() -> Self {
        Self {
            filter_allows_components: false,
            filter: Default::default(),
            clone_behavior_overrides: Default::default(),
            move_components: false,
            is_recursive: false,
            default_clone_fn: ComponentCloneBehavior::global_default_fn(),
            clone_queue: Default::default(),
        }
    }
}

/// An expandable scratch space for defining a dynamic bundle.
struct BundleScratch<'a> {
    component_ids: Vec<ComponentId>,
    component_ptrs: Vec<PtrMut<'a>>,
}

impl<'a> BundleScratch<'a> {
    pub(crate) fn with_capacity(capacity: usize) -> Self {
        Self {
            component_ids: Vec::with_capacity(capacity),
            component_ptrs: Vec::with_capacity(capacity),
        }
    }

    /// Pushes the `ptr` component onto this storage with the given `id` [`ComponentId`].
    ///
    /// # Safety
    /// The `id` [`ComponentId`] must match the component `ptr` for whatever [`World`] this scratch will
    /// be written to. `ptr` must contain valid uniquely-owned data that matches the type of component referenced
    /// in `id`.
    pub(crate) unsafe fn push_ptr(&mut self, id: ComponentId, ptr: PtrMut<'a>) {
        self.component_ids.push(id);
        self.component_ptrs.push(ptr);
    }

    /// Pushes the `C` component onto this storage with the given `id` [`ComponentId`], using the given `bump` allocator.
    ///
    /// # Safety
    /// The `id` [`ComponentId`] must match the component `C` for whatever [`World`] this scratch will
    /// be written to.
    pub(crate) unsafe fn push<C: Component>(
        &mut self,
        allocator: &'a Bump,
        id: ComponentId,
        component: C,
    ) {
        let component_ref = allocator.alloc(component);
        self.component_ids.push(id);
        self.component_ptrs.push(PtrMut::from(component_ref));
    }

    /// Writes the scratch components to the given entity in the given world.
    ///
    /// # Safety
    /// All [`ComponentId`] values in this instance must come from `world`.
    pub(crate) unsafe fn write(self, world: &mut World, entity: Entity) {
        // SAFETY:
        // - All `component_ids` are from the same world as `target` entity
        // - All `component_data_ptrs` are valid types represented by `component_ids`
        unsafe {
            world.entity_mut(entity).insert_by_ids(
                &self.component_ids,
                self.component_ptrs.into_iter().map(|ptr| ptr.promote()),
            );
        }
    }
}

impl EntityCloner {
    /// Returns a new [`EntityClonerBuilder`] using the given `world`.
    pub fn build(world: &mut World) -> EntityClonerBuilder {
        EntityClonerBuilder {
            world,
            attach_required_components: true,
            entity_cloner: EntityCloner::default(),
        }
    }

    /// Returns `true` if this cloner is configured to clone entities recursively.
    #[inline]
    pub fn is_recursive(&self) -> bool {
        self.is_recursive
    }

    /// Clones and inserts components from the `source` entity into the entity mapped by `mapper` from `source` using the stored configuration.
    fn clone_entity_internal(
        &mut self,
        world: &mut World,
        source: Entity,
        mapper: &mut dyn EntityMapper,
    ) -> Entity {
        let target = mapper.get_mapped(source);
        // PERF: reusing allocated space across clones would be more efficient. Consider an allocation model similar to `Commands`.
        let bundle_scratch_allocator = Bump::new();
        let mut bundle_scratch: BundleScratch;
        {
            let world = world.as_unsafe_world_cell();
            let source_entity = world.get_entity(source).expect("Source entity must exist");

            #[cfg(feature = "bevy_reflect")]
            // SAFETY: we have unique access to `world`, nothing else accesses the registry at this moment, and we clone
            // the registry, which prevents future conflicts.
            let app_registry = unsafe {
                world
                    .get_resource::<crate::reflect::AppTypeRegistry>()
                    .cloned()
            };
            #[cfg(not(feature = "bevy_reflect"))]
            let app_registry = Option::<()>::None;

            let archetype = source_entity.archetype();
            bundle_scratch = BundleScratch::with_capacity(archetype.component_count());
            // SAFETY: no other references to command queue exist
            let mut commands = unsafe {
                Commands::new_raw_from_entities(world.get_raw_command_queue(), world.entities())
            };

            for component in archetype.components() {
                if !self.is_cloning_allowed(&component) {
                    continue;
                }

                let handler = match self.clone_behavior_overrides.get(&component) {
                    Some(clone_behavior) => clone_behavior.resolve(self.default_clone_fn),
                    None => world
                        .components()
                        .get_info(component)
                        .map(|info| info.clone_behavior().resolve(self.default_clone_fn))
                        .unwrap_or(self.default_clone_fn),
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
                        source,
                        target,
                        source_component_ptr,
                        &bundle_scratch_allocator,
                        &mut bundle_scratch,
                        world.components(),
                        self,
                        mapper,
                        app_registry.as_ref(),
                    )
                };

                (handler)(&mut commands, &mut ctx);
            }
        }

        world.flush();

        if !world.entities.contains(target) {
            panic!("Target entity does not exist");
        }

        if self.move_components {
            world
                .entity_mut(source)
                .remove_by_ids(&bundle_scratch.component_ids);
        }

        // SAFETY:
        // - All `component_ids` are from the same world as `target` entity
        // - All `component_data_ptrs` are valid types represented by `component_ids`
        unsafe { bundle_scratch.write(world, target) };
        target
    }

    /// Clones and inserts components from the `source` entity into `target` entity using the stored configuration.
    /// If this [`EntityCloner`] has [`EntityCloner::is_recursive`], then it will recursively spawn entities as defined
    /// by [`RelationshipTarget`](crate::relationship::RelationshipTarget) components with
    /// [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN)
    #[track_caller]
    pub fn clone_entity(&mut self, world: &mut World, source: Entity, target: Entity) {
        let mut map = EntityHashMap::<Entity>::new();
        map.set_mapped(source, target);
        self.clone_entity_mapped(world, source, &mut map);
    }

    /// Clones and inserts components from the `source` entity into a newly spawned entity using the stored configuration.
    /// If this [`EntityCloner`] has [`EntityCloner::is_recursive`], then it will recursively spawn entities as defined
    /// by [`RelationshipTarget`](crate::relationship::RelationshipTarget) components with
    /// [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN)
    #[track_caller]
    pub fn spawn_clone(&mut self, world: &mut World, source: Entity) -> Entity {
        let target = world.spawn_empty().id();
        self.clone_entity(world, source, target);
        target
    }

    /// Clones the entity into whatever entity `mapper` chooses for it.
    #[track_caller]
    pub fn clone_entity_mapped(
        &mut self,
        world: &mut World,
        source: Entity,
        mapper: &mut dyn EntityMapper,
    ) -> Entity {
        let target = self.clone_entity_internal(world, source, mapper);
        loop {
            let queued = self.clone_queue.borrow_mut().pop_front();
            if let Some(queued) = queued {
                let target = world.entities.reserve_entity();
                mapper.set_mapped(queued, target);
                self.clone_entity_internal(world, queued, mapper);
            } else {
                break;
            }
        }
        target
    }

    fn is_cloning_allowed(&self, component: &ComponentId) -> bool {
        (self.filter_allows_components && self.filter.contains(component))
            || (!self.filter_allows_components && !self.filter.contains(component))
    }
}

/// A builder for configuring [`EntityCloner`]. See [`EntityCloner`] for more information.
#[derive(Debug)]
pub struct EntityClonerBuilder<'w> {
    world: &'w mut World,
    entity_cloner: EntityCloner,
    attach_required_components: bool,
}

impl<'w> EntityClonerBuilder<'w> {
    /// Internally calls [`EntityCloner::clone_entity`] on the builder's [`World`].
    pub fn clone_entity(&mut self, source: Entity, target: Entity) -> &mut Self {
        self.entity_cloner.clone_entity(self.world, source, target);
        self
    }
    /// Finishes configuring [`EntityCloner`] returns it.
    pub fn finish(self) -> EntityCloner {
        self.entity_cloner
    }

    /// By default, any components allowed/denied through the filter will automatically
    /// allow/deny all of their required components.
    ///
    /// This method allows for a scoped mode where any changes to the filter
    /// will not involve required components.
    pub fn without_required_components(
        &mut self,
        builder: impl FnOnce(&mut EntityClonerBuilder) + Send + Sync + 'static,
    ) -> &mut Self {
        self.attach_required_components = false;
        builder(self);
        self.attach_required_components = true;
        self
    }

    /// Sets the default clone function to use.
    pub fn with_default_clone_fn(&mut self, clone_fn: ComponentCloneFn) -> &mut Self {
        self.entity_cloner.default_clone_fn = clone_fn;
        self
    }

    /// Sets whether the cloner should remove any components that were cloned,
    /// effectively moving them from the source entity to the target.
    ///
    /// This is disabled by default.
    ///
    /// The setting only applies to components that are allowed through the filter
    /// at the time [`EntityClonerBuilder::clone_entity`] is called.
    pub fn move_components(&mut self, enable: bool) -> &mut Self {
        self.entity_cloner.move_components = enable;
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
        self.entity_cloner.filter_allows_components = false;
        self.entity_cloner.filter.clear();
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
        self.entity_cloner.filter_allows_components = true;
        self.entity_cloner.filter.clear();
        self
    }

    /// Overrides the [`ComponentCloneBehavior`] for a component in this builder.
    /// This handler will be used to clone the component instead of the global one defined by the [`EntityCloner`].
    ///
    /// See [Handlers section of `EntityClonerBuilder`](EntityClonerBuilder#handlers) to understand how this affects handler priority.
    pub fn override_clone_behavior<T: Component>(
        &mut self,
        clone_behavior: ComponentCloneBehavior,
    ) -> &mut Self {
        if let Some(id) = self.world.components().component_id::<T>() {
            self.entity_cloner
                .clone_behavior_overrides
                .insert(id, clone_behavior);
        }
        self
    }

    /// Overrides the [`ComponentCloneBehavior`] for a component with the given `component_id` in this builder.
    /// This handler will be used to clone the component instead of the global one defined by the [`EntityCloner`].
    ///
    /// See [Handlers section of `EntityClonerBuilder`](EntityClonerBuilder#handlers) to understand how this affects handler priority.
    pub fn override_clone_behavior_with_id(
        &mut self,
        component_id: ComponentId,
        clone_behavior: ComponentCloneBehavior,
    ) -> &mut Self {
        self.entity_cloner
            .clone_behavior_overrides
            .insert(component_id, clone_behavior);
        self
    }

    /// Removes a previously set override of [`ComponentCloneBehavior`] for a component in this builder.
    pub fn remove_clone_behavior_override<T: Component>(&mut self) -> &mut Self {
        if let Some(id) = self.world.components().component_id::<T>() {
            self.entity_cloner.clone_behavior_overrides.remove(&id);
        }
        self
    }

    /// Removes a previously set override of [`ComponentCloneBehavior`] for a given `component_id` in this builder.
    pub fn remove_clone_behavior_override_with_id(
        &mut self,
        component_id: ComponentId,
    ) -> &mut Self {
        self.entity_cloner
            .clone_behavior_overrides
            .remove(&component_id);
        self
    }

    /// If `true`, makes the built [`EntityCloner`] recursively clone entities, as defined by
    /// [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN).
    pub fn recursive(&mut self, is_recursive: bool) -> &mut Self {
        self.entity_cloner.is_recursive = is_recursive;
        self
    }

    /// Helper function that allows a component through the filter.
    fn filter_allow(&mut self, id: ComponentId) {
        if self.entity_cloner.filter_allows_components {
            self.entity_cloner.filter.insert(id);
        } else {
            self.entity_cloner.filter.remove(&id);
        }
        if self.attach_required_components {
            if let Some(info) = self.world.components().get_info(id) {
                for required_id in info.required_components().iter_ids() {
                    if self.entity_cloner.filter_allows_components {
                        self.entity_cloner.filter.insert(required_id);
                    } else {
                        self.entity_cloner.filter.remove(&required_id);
                    }
                }
            }
        }
    }

    /// Helper function that disallows a component through the filter.
    fn filter_deny(&mut self, id: ComponentId) {
        if self.entity_cloner.filter_allows_components {
            self.entity_cloner.filter.remove(&id);
        } else {
            self.entity_cloner.filter.insert(id);
        }
        if self.attach_required_components {
            if let Some(info) = self.world.components().get_info(id) {
                for required_id in info.required_components().iter_ids() {
                    if self.entity_cloner.filter_allows_components {
                        self.entity_cloner.filter.remove(&required_id);
                    } else {
                        self.entity_cloner.filter.insert(required_id);
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
        component::{Component, ComponentCloneBehavior, ComponentDescriptor, StorageType},
        entity::{hash_map::EntityHashMap, Entity, EntityCloner},
        prelude::{ChildOf, Children, Resource},
        reflect::AppTypeRegistry,
        reflect::{ReflectComponent, ReflectFromWorld},
        system::Commands,
        world::{FromWorld, World},
    };
    use alloc::vec::Vec;
    use bevy_ecs_macros::require;
    use bevy_ptr::OwningPtr;
    use bevy_reflect::Reflect;
    use core::{alloc::Layout, ops::Deref};

    #[cfg(feature = "bevy_reflect")]
    mod reflect {
        use super::*;
        use crate::{
            component::{Component, ComponentCloneBehavior},
            entity::EntityCloner,
            reflect::{AppTypeRegistry, ReflectComponent, ReflectFromWorld},
            system::Commands,
        };
        use alloc::vec;
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
            let component = A { field: 5 };

            let e = world.spawn(component.clone()).id();
            let e_clone = world.spawn_empty().id();

            EntityCloner::build(&mut world)
                .override_clone_behavior::<A>(ComponentCloneBehavior::reflect())
                .clone_entity(e, e_clone);

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

            EntityCloner::build(&mut world)
                .override_clone_behavior_with_id(a_id, ComponentCloneBehavior::reflect())
                .override_clone_behavior_with_id(b_id, ComponentCloneBehavior::reflect())
                .override_clone_behavior_with_id(c_id, ComponentCloneBehavior::reflect())
                .clone_entity(e, e_clone);

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

            fn test_handler(_commands: &mut Commands, ctx: &mut ComponentCloneCtx) {
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

            let e = world.spawn(A).id();
            let e_clone = world.spawn_empty().id();

            EntityCloner::build(&mut world)
                .override_clone_behavior::<A>(ComponentCloneBehavior::Custom(test_handler))
                .clone_entity(e, e_clone);
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

            EntityCloner::build(&mut world).clone_entity(e, e_clone);

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

            // No AppTypeRegistry
            let e = world.spawn((A, B)).id();
            let e_clone = world.spawn_empty().id();
            EntityCloner::build(&mut world)
                .override_clone_behavior::<A>(ComponentCloneBehavior::reflect())
                .override_clone_behavior::<B>(ComponentCloneBehavior::reflect())
                .clone_entity(e, e_clone);
            assert_eq!(world.get::<A>(e_clone), None);
            assert_eq!(world.get::<B>(e_clone), None);

            // With AppTypeRegistry
            world.init_resource::<AppTypeRegistry>();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            registry.write().register::<B>();

            let e = world.spawn((A, B)).id();
            let e_clone = world.spawn_empty().id();
            EntityCloner::build(&mut world).clone_entity(e, e_clone);
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

        EntityCloner::build(&mut world).clone_entity(e, e_clone);

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

        EntityCloner::build(&mut world)
            .deny_all()
            .allow::<A>()
            .clone_entity(e, e_clone);

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

        EntityCloner::build(&mut world)
            .deny::<B>()
            .clone_entity(e, e_clone);

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

        EntityCloner::build(&mut world)
            .deny_all()
            .allow::<A>()
            .allow::<B>()
            .allow::<C>()
            .deny::<B>()
            .clone_entity(e, e_clone);

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

        EntityCloner::build(&mut world)
            .deny_all()
            .allow::<(A, B, C)>()
            .deny::<(B, C)>()
            .clone_entity(e, e_clone);

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

        EntityCloner::build(&mut world)
            .deny_all()
            .without_required_components(|builder| {
                builder.allow::<B>();
            })
            .clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), None);
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B));
        assert_eq!(world.entity(e_clone).get::<C>(), Some(&C(5)));
    }

    #[test]
    fn clone_entity_with_dynamic_components() {
        const COMPONENT_SIZE: usize = 10;
        fn test_handler(_commands: &mut Commands, ctx: &mut ComponentCloneCtx) {
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
                ComponentCloneBehavior::Custom(test_handler),
            )
        };
        let component_id = world.register_component_with_descriptor(descriptor);

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
        EntityCloner::build(&mut world).clone_entity(entity, entity_clone);

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

    #[test]
    fn recursive_clone() {
        let mut world = World::new();
        let root = world.spawn_empty().id();
        let child1 = world.spawn(ChildOf(root)).id();
        let grandchild = world.spawn(ChildOf(child1)).id();
        let child2 = world.spawn(ChildOf(root)).id();

        let clone_root = world.spawn_empty().id();
        EntityCloner::build(&mut world)
            .recursive(true)
            .clone_entity(root, clone_root);

        let root_children = world
            .entity(clone_root)
            .get::<Children>()
            .unwrap()
            .iter()
            .cloned()
            .collect::<Vec<_>>();

        assert!(root_children.iter().all(|e| *e != child1 && *e != child2));
        assert_eq!(root_children.len(), 2);
        let child1_children = world.entity(root_children[0]).get::<Children>().unwrap();
        assert_eq!(child1_children.len(), 1);
        assert_ne!(child1_children[0], grandchild);
        assert!(world.entity(root_children[1]).get::<Children>().is_none());

        assert_eq!(
            world.entity(root).get::<Children>().unwrap().deref(),
            &[child1, child2]
        );
    }

    #[test]
    fn clone_with_reflect_from_world() {
        #[derive(Component, Reflect, PartialEq, Eq, Debug)]
        #[reflect(Component, FromWorld, from_reflect = false)]
        struct SomeRef(#[entities] Entity);

        #[derive(Resource)]
        struct FromWorldCalled(bool);

        impl FromWorld for SomeRef {
            fn from_world(world: &mut World) -> Self {
                world.insert_resource(FromWorldCalled(true));
                SomeRef(Entity::PLACEHOLDER)
            }
        }
        let mut world = World::new();
        let registry = AppTypeRegistry::default();
        registry.write().register::<SomeRef>();
        world.insert_resource(registry);

        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn(SomeRef(a)).id();
        let d = world.spawn_empty().id();
        let mut map = EntityHashMap::<Entity>::new();
        map.insert(a, b);
        map.insert(c, d);

        let cloned = EntityCloner::default().clone_entity_mapped(&mut world, c, &mut map);
        assert_eq!(*world.entity(cloned).get::<SomeRef>().unwrap(), SomeRef(b));
        assert!(world.resource::<FromWorldCalled>().0);
    }
}
