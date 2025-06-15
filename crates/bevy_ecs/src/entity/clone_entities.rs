use alloc::{borrow::ToOwned, boxed::Box, collections::VecDeque, vec::Vec};
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_ptr::{Ptr, PtrMut};
use bumpalo::Bump;
use core::{any::TypeId, ops::Range};

use crate::{
    archetype::Archetype,
    bundle::{Bundle, BundleId, InsertMode},
    component::{Component, ComponentCloneBehavior, ComponentCloneFn, ComponentId, ComponentInfo},
    entity::{hash_map::EntityHashMap, Entities, Entity, EntityMapper},
    query::DebugCheckedUnwrap,
    relationship::RelationshipHookMode,
    world::World,
};

/// Provides read access to the source component (the component being cloned) in a [`ComponentCloneFn`].
pub struct SourceComponent<'a> {
    ptr: Ptr<'a>,
    info: &'a ComponentInfo,
}

impl<'a> SourceComponent<'a> {
    /// Returns a reference to the component on the source entity.
    ///
    /// Will return `None` if `ComponentId` of requested component does not match `ComponentId` of source component
    pub fn read<C: Component>(&self) -> Option<&C> {
        if self
            .info
            .type_id()
            .is_some_and(|id| id == TypeId::of::<C>())
        {
            // SAFETY:
            // - Components and ComponentId are from the same world
            // - source_component_ptr holds valid data of the type referenced by ComponentId
            unsafe { Some(self.ptr.deref::<C>()) }
        } else {
            None
        }
    }

    /// Returns the "raw" pointer to the source component.
    pub fn ptr(&self) -> Ptr<'a> {
        self.ptr
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
    pub fn read_reflect(
        &self,
        registry: &bevy_reflect::TypeRegistry,
    ) -> Option<&dyn bevy_reflect::Reflect> {
        let type_id = self.info.type_id()?;
        let reflect_from_ptr = registry.get_type_data::<bevy_reflect::ReflectFromPtr>(type_id)?;
        if reflect_from_ptr.type_id() != type_id {
            return None;
        }
        // SAFETY: `source_component_ptr` stores data represented by `component_id`, which we used to get `ReflectFromPtr`.
        unsafe { Some(reflect_from_ptr.as_reflect(self.ptr)) }
    }
}

/// Context for component clone handlers.
///
/// Provides fast access to useful resources like [`AppTypeRegistry`](crate::reflect::AppTypeRegistry)
/// and allows component clone handler to get information about component being cloned.
pub struct ComponentCloneCtx<'a, 'b> {
    component_id: ComponentId,
    target_component_written: bool,
    bundle_scratch: &'a mut BundleScratch<'b>,
    bundle_scratch_allocator: &'b Bump,
    entities: &'a Entities,
    source: Entity,
    target: Entity,
    component_info: &'a ComponentInfo,
    entity_cloner: &'a mut EntityCloner,
    mapper: &'a mut dyn EntityMapper,
    #[cfg(feature = "bevy_reflect")]
    type_registry: Option<&'a crate::reflect::AppTypeRegistry>,
    #[cfg(not(feature = "bevy_reflect"))]
    #[expect(dead_code, reason = "type_registry is only used with bevy_reflect")]
    type_registry: Option<&'a ()>,
}

impl<'a, 'b> ComponentCloneCtx<'a, 'b> {
    /// Create a new instance of `ComponentCloneCtx` that can be passed to component clone handlers.
    ///
    /// # Safety
    /// Caller must ensure that:
    /// - `component_info` corresponds to the `component_id` in the same world,.
    /// - `source_component_ptr` points to a valid component of type represented by `component_id`.
    unsafe fn new(
        component_id: ComponentId,
        source: Entity,
        target: Entity,
        bundle_scratch_allocator: &'b Bump,
        bundle_scratch: &'a mut BundleScratch<'b>,
        entities: &'a Entities,
        component_info: &'a ComponentInfo,
        entity_cloner: &'a mut EntityCloner,
        mapper: &'a mut dyn EntityMapper,
        #[cfg(feature = "bevy_reflect")] type_registry: Option<&'a crate::reflect::AppTypeRegistry>,
        #[cfg(not(feature = "bevy_reflect"))] type_registry: Option<&'a ()>,
    ) -> Self {
        Self {
            component_id,
            source,
            target,
            bundle_scratch,
            target_component_written: false,
            bundle_scratch_allocator,
            entities,
            mapper,
            component_info,
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
    pub fn linked_cloning(&self) -> bool {
        self.entity_cloner.linked_cloning
    }

    /// Returns this context's [`EntityMapper`].
    pub fn entity_mapper(&mut self) -> &mut dyn EntityMapper {
        self.mapper
    }

    /// Writes component data to target entity.
    ///
    /// # Panics
    /// This will panic if:
    /// - Component has already been written once.
    /// - Component being written is not registered in the world.
    /// - `ComponentId` of component being written does not match expected `ComponentId`.
    pub fn write_target_component<C: Component>(&mut self, mut component: C) {
        C::map_entities(&mut component, &mut self.mapper);
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

    /// Writes component data to target entity by providing a pointer to source component data.
    ///
    /// # Safety
    /// Caller must ensure that the passed in `ptr` references data that corresponds to the type of the source / target [`ComponentId`].
    /// `ptr` must also contain data that the written component can "own" (for example, this should not directly copy non-Copy data).
    ///
    /// # Panics
    /// This will panic if component has already been written once.
    pub unsafe fn write_target_component_ptr(&mut self, ptr: Ptr) {
        if self.target_component_written {
            panic!("Trying to write component multiple times")
        }
        let layout = self.component_info.layout();
        let target_ptr = self.bundle_scratch_allocator.alloc_layout(layout);
        core::ptr::copy_nonoverlapping(ptr.as_ptr(), target_ptr.as_ptr(), layout.size());
        self.bundle_scratch
            .push_ptr(self.component_id, PtrMut::new(target_ptr));
        self.target_component_written = true;
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

            if component_layout.size() > 0 {
                // Ensure we don't attempt to deallocate zero-sized components
                alloc::alloc::dealloc(component_data_ptr, component_layout);
            }
        }

        self.target_component_written = true;
    }

    /// Returns [`AppTypeRegistry`](`crate::reflect::AppTypeRegistry`) if it exists in the world.
    ///
    /// NOTE: Prefer this method instead of manually reading the resource from the world.
    #[cfg(feature = "bevy_reflect")]
    pub fn type_registry(&self) -> Option<&crate::reflect::AppTypeRegistry> {
        self.type_registry
    }

    /// Queues the `entity` to be cloned by the current [`EntityCloner`]
    pub fn queue_entity_clone(&mut self, entity: Entity) {
        let target = self.entities.reserve_entity();
        self.mapper.set_mapped(entity, target);
        self.entity_cloner.clone_queue.push_back(entity);
    }

    /// Queues a deferred clone operation, which will run with exclusive [`World`] access immediately after calling the clone handler for each component on an entity.
    /// This exists, despite its similarity to [`Commands`](crate::system::Commands), to provide access to the entity mapper in the current context.
    pub fn queue_deferred(
        &mut self,
        deferred: impl FnOnce(&mut World, &mut dyn EntityMapper) + 'static,
    ) {
        self.entity_cloner
            .deferred_commands
            .push_back(Box::new(deferred));
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
/// #[derive(Clone, Component)]
/// #[component(clone_behavior = clone::<Self>())]
/// struct SomeComponent;
///
/// ```
///
/// # Clone Behaviors
/// [`EntityCloner`] clones entities by cloning components using [`ComponentCloneBehavior`], and there are multiple layers
/// to decide which handler to use for which component. The overall hierarchy looks like this (priority from most to least):
/// 1. local overrides using [`EntityClonerBuilder::override_clone_behavior`]
/// 2. component-defined handler using [`Component::clone_behavior`]
/// 3. default handler override using [`EntityClonerBuilder::with_default_clone_fn`].
/// 4. reflect-based or noop default clone handler depending on if `bevy_reflect` feature is enabled or not.
pub struct EntityCloner {
    filter: Option<AllowOrDenyAll>,
    clone_behavior_overrides: HashMap<ComponentId, ComponentCloneBehavior>,
    move_components: bool,
    linked_cloning: bool,
    default_clone_fn: ComponentCloneFn,
    clone_queue: VecDeque<Entity>,
    deferred_commands: VecDeque<Box<dyn FnOnce(&mut World, &mut dyn EntityMapper)>>,
}

impl Default for EntityCloner {
    fn default() -> Self {
        Self {
            filter: Some(AllowOrDenyAll::AllowAll(AllowAll {
                deny: Default::default(),
            })),
            move_components: false,
            linked_cloning: false,
            default_clone_fn: ComponentCloneBehavior::global_default_fn(),
            clone_behavior_overrides: Default::default(),
            clone_queue: Default::default(),
            deferred_commands: Default::default(),
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
    pub(crate) unsafe fn write(
        self,
        world: &mut World,
        entity: Entity,
        relationship_hook_insert_mode: RelationshipHookMode,
    ) {
        // SAFETY:
        // - All `component_ids` are from the same world as `target` entity
        // - All `component_data_ptrs` are valid types represented by `component_ids`
        unsafe {
            world.entity_mut(entity).insert_by_ids_internal(
                &self.component_ids,
                self.component_ptrs.into_iter().map(|ptr| ptr.promote()),
                relationship_hook_insert_mode,
            );
        }
    }
}

impl EntityCloner {
    /// Returns a new [`EntityClonerBuilder`] using the given `world` with the [`AllowAll`] configuration.
    ///
    /// This builder tries to clone every component from the source entity except for components that were
    /// explicitly denied, for example by using the [`deny`](EntityClonerBuilder<AllowAll>::deny) method.
    ///
    /// Required components are not considered by denied components and must be explicitly denied as well if desired.
    pub fn build_allow_all(world: &mut World) -> EntityClonerBuilder<AllowAll> {
        EntityClonerBuilder {
            world,
            entity_cloner: EntityCloner {
                filter: None, // to be inserted from `EntityClonerBuilder::filter`
                ..Default::default()
            },
            filter: Default::default(),
        }
    }

    /// Returns a new [`EntityClonerBuilder`] using the given `world` with the [`DenyAll`] configuration.
    ///
    /// This builder tries to clone every component that was explicitly allowed from the source entity,
    /// for example by using the [`allow`](EntityClonerBuilder<DenyAll>::allow) method.
    ///
    /// Required components are also cloned when the target entity does not contain them.
    pub fn build_deny_all(world: &mut World) -> EntityClonerBuilder<DenyAll> {
        EntityClonerBuilder {
            world,
            entity_cloner: EntityCloner {
                filter: None, // to be inserted from `EntityClonerBuilder::filter`
                ..Default::default()
            },
            filter: Default::default(),
        }
    }

    /// Returns `true` if this cloner is configured to clone entities referenced in cloned components via [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN).
    /// This will produce "deep" / recursive clones of relationship trees that have "linked spawn".
    #[inline]
    pub fn linked_cloning(&self) -> bool {
        self.linked_cloning
    }

    /// Clones and inserts components from the `source` entity into the entity mapped by `mapper` from `source` using the stored configuration.
    fn clone_entity_internal(
        &mut self,
        world: &mut World,
        source: Entity,
        mapper: &mut dyn EntityMapper,
        relationship_hook_insert_mode: RelationshipHookMode,
    ) -> Entity {
        let target = mapper.get_mapped(source);
        // PERF: reusing allocated space across clones would be more efficient. Consider an allocation model similar to `Commands`.
        let bundle_scratch_allocator = Bump::new();
        let mut bundle_scratch: BundleScratch;
        {
            let world = world.as_unsafe_world_cell();
            let source_entity = world.get_entity(source).expect("Source entity must exist");
            let mut filter = self.filter.take().expect("todo");

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

            let source_archetype = source_entity.archetype();
            bundle_scratch = BundleScratch::with_capacity(source_archetype.component_count());

            let mut clone_component = |component: ComponentId, cloner: &mut EntityCloner| {
                let handler = match cloner.clone_behavior_overrides.get(&component) {
                    Some(clone_behavior) => clone_behavior.resolve(cloner.default_clone_fn),
                    None => world
                        .components()
                        .get_info(component)
                        .map(|info| info.clone_behavior().resolve(cloner.default_clone_fn))
                        .unwrap_or(cloner.default_clone_fn),
                };

                // SAFETY: This component exists because it is present on the archetype.
                let info = unsafe { world.components().get_info_unchecked(component) };

                // SAFETY:
                // - There are no other mutable references to source entity.
                // - `component` is from `source_entity`'s archetype
                let source_component_ptr =
                    unsafe { source_entity.get_by_id(component).debug_checked_unwrap() };

                let source_component = SourceComponent {
                    info,
                    ptr: source_component_ptr,
                };

                // SAFETY:
                // - `components` and `component` are from the same world
                // - `source_component_ptr` is valid and points to the same type as represented by `component`
                let mut ctx = unsafe {
                    ComponentCloneCtx::new(
                        component,
                        source,
                        target,
                        &bundle_scratch_allocator,
                        &mut bundle_scratch,
                        world.entities(),
                        info,
                        cloner,
                        mapper,
                        app_registry.as_ref(),
                    )
                };

                (handler)(&source_component, &mut ctx);
            };

            match &mut filter {
                AllowOrDenyAll::DenyAll(filter) => {
                    let target_archetype = filter.needs_target_archetype.then(|| {
                        world
                            .get_entity(target)
                            .expect("Target entity must exist")
                            .archetype()
                    });
                    // SAFETY: `target_archetype` is Some based on `filter.needs_target_archetype`
                    unsafe {
                        filter.iter_components(source_archetype, target_archetype, |component| {
                            clone_component(component, self)
                        })
                    };
                }
                AllowOrDenyAll::AllowAll(filter) => {
                    for component in source_archetype.components() {
                        if !filter.deny.contains(&component) {
                            clone_component(component, self);
                        }
                    }
                }
            }

            self.filter = Some(filter);
        }

        world.flush();

        for deferred in self.deferred_commands.drain(..) {
            (deferred)(world, mapper);
        }

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
        unsafe { bundle_scratch.write(world, target, relationship_hook_insert_mode) };
        target
    }

    /// Clones and inserts components from the `source` entity into `target` entity using the stored configuration.
    /// If this [`EntityCloner`] has [`EntityCloner::linked_cloning`], then it will recursively spawn entities as defined
    /// by [`RelationshipTarget`](crate::relationship::RelationshipTarget) components with
    /// [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN)
    #[track_caller]
    pub fn clone_entity(&mut self, world: &mut World, source: Entity, target: Entity) {
        let mut map = EntityHashMap::<Entity>::new();
        map.set_mapped(source, target);
        self.clone_entity_mapped(world, source, &mut map);
    }

    /// Clones and inserts components from the `source` entity into a newly spawned entity using the stored configuration.
    /// If this [`EntityCloner`] has [`EntityCloner::linked_cloning`], then it will recursively spawn entities as defined
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
        // All relationships on the root should have their hooks run
        let target = self.clone_entity_internal(world, source, mapper, RelationshipHookMode::Run);
        let child_hook_insert_mode = if self.linked_cloning {
            // When spawning "linked relationships", we want to ignore hooks for relationships we are spawning, while
            // still registering with original relationship targets that are "not linked" to the current recursive spawn.
            RelationshipHookMode::RunIfNotLinked
        } else {
            // If we are not cloning "linked relationships" recursively, then we want any cloned relationship components to
            // register themselves with their original relationship target.
            RelationshipHookMode::Run
        };
        loop {
            let queued = self.clone_queue.pop_front();
            if let Some(queued) = queued {
                self.clone_entity_internal(world, queued, mapper, child_hook_insert_mode);
            } else {
                break;
            }
        }
        target
    }
}

enum AllowOrDenyAll {
    DenyAll(DenyAll),
    AllowAll(AllowAll),
}

/// Generic for [`EntityClonerBuilder`] that makes the cloner try to clone every component from the source entity
/// except for components that were explicitly denied, for example by using the
/// [`deny`](EntityClonerBuilder::deny) method.
///
/// Required components are not considered by denied components and must be explicitly denied as well if desired.
#[derive(Default)]
pub struct AllowAll {
    /// Contains the components that should not be cloned.
    deny: HashSet<ComponentId>,
}

/// Generic for [`EntityClonerBuilder`] that makes the cloner try to clone every component that was explicitly
/// allowed from the source entity, for example by using the [`allow`](EntityClonerBuilder::allow) method.
///
/// Required components are also cloned when the target entity does not contain them.
#[derive(Default)]
pub struct DenyAll {
    /// Contains the components explicitly allowed to be cloned.
    explicits: HashMap<ComponentId, Explicit>,

    /// Lists of required components, [`Explicit`] refers to a range in it.
    requires_of_explicits: Vec<ComponentId>,

    /// Contains the components required by those in [`Self::explicits`].
    /// Also contains the number of components in [`Self::explicits`] each is required by to track
    /// when to skip cloning a required component after skipping explicit components that require it.
    ///
    /// If this is not empty, [`Self::needs_target_archetype`] is true.
    requires: HashMap<ComponentId, Required>,

    /// Is `true` when the target [`Archetype`] is needed as required components and those configured
    /// as allowed "if new" should only be cloned when absent at the target.
    needs_target_archetype: bool,

    /// Is `true` unless during [`EntityClonerBuilder::without_required_components`] which will suppress
    /// evaluating required components to clone, causing them to be created independent from the value at
    /// the source entity if needed.
    attach_required_components: bool,
}

/// Contains the components explicitly allowed to be cloned.
struct Explicit {
    /// If component was added via [`allow`](EntityClonerBuilder::allow) etc, this is `Overwrite`.
    ///
    /// If component was added via [`allow_if_new`](EntityClonerBuilder::allow_if_new) etc, this is `Keep`.
    ///
    /// If any `Explicit` exists with `Keep`, then [`DenyAll::needs_target_archetype`] is `true`.
    insert_mode: InsertMode,

    /// Contains the range in [`DenyAll::requires_of_explicits`] for this component containing its
    /// required components.
    ///
    /// Is `None` if [`DenyAll::attach_required_components`] was `false` when added.
    /// It may be set to `Some` later if the component is later added explicitly again with
    /// [`DenyAll::attach_required_components`] being `true`.
    ///
    /// Range is empty if this component has no required components.
    requires: Option<Range<usize>>,
}

struct Required {
    /// Amount of explicit components this component is required by.
    required_by: usize,

    /// As [`Self::required_by`] but is reduced during cloning when an explicit component is not cloned,
    /// either because [`Explicit::insert_mode`] is `Keep` or the source entity does not contain it.
    ///
    /// If this is zero, the required component is not cloned.
    ///
    /// The counter is reset to `required_by` when the cloning is over in case another entity needs to be
    /// cloned by the same [`EntityCloner`].
    required_by_reduced: usize,
}

impl DenyAll {
    /// Helper function that allows a component through the filter.
    fn filter_allow(&mut self, id: ComponentId, world: &World, insert_mode: InsertMode) {
        // extend `Self::requires_of_explicits` with required components in `info` and return inserted range
        let mut update_requires_get_range = |info: &ComponentInfo| -> Range<usize> {
            let iter = info.required_components().iter_ids().inspect(|id| {
                // set or increase the number of components this `id` is required by
                self.requires
                    .entry(*id)
                    .and_modify(|required| {
                        required.required_by += 1;
                        required.required_by_reduced += 1;
                    })
                    .or_insert(Required {
                        required_by: 1,
                        required_by_reduced: 1,
                    });
            });

            let start = self.requires_of_explicits.len();
            self.requires_of_explicits.extend(iter);
            let end = self.requires_of_explicits.len();

            start..end
        };

        match self.explicits.entry(id) {
            Entry::Vacant(vacant) => {
                if !self.attach_required_components {
                    vacant.insert(Explicit {
                        insert_mode,
                        requires: None,
                    });

                    if insert_mode == InsertMode::Keep {
                        // need to know target archetype to insert explicit only if new
                        self.needs_target_archetype = true;
                    }
                } else if let Some(info) = world.components().get_info(id) {
                    // need to know target archetype to insert required only if new
                    self.needs_target_archetype = true;

                    vacant.insert(Explicit {
                        insert_mode,
                        requires: Some(update_requires_get_range(info)),
                    });
                }
            }
            Entry::Occupied(mut occupied) => {
                let explicit = occupied.get_mut();

                match insert_mode {
                    InsertMode::Replace => {
                        // overwrite value if it was inserted as `Keep` earlier
                        explicit.insert_mode = InsertMode::Replace;
                    }
                    InsertMode::Keep => {
                        if explicit.insert_mode == InsertMode::Keep {
                            // need to know target archetype to insert explicit only if new
                            self.needs_target_archetype = true;
                        }
                    }
                }

                // set required component range if it was inserted with `None` earlier
                if self.attach_required_components && explicit.requires.is_none() {
                    if let Some(info) = world.components().get_info(id) {
                        // need to know target archetype to insert required only if new
                        self.needs_target_archetype = true;

                        explicit.requires = Some(update_requires_get_range(info));
                    }
                }
            }
        };
    }

    /// Call `clone_component` for all components to be cloned.
    ///
    /// # Safety
    ///
    /// `target_archetype` must be `Some` when [`Self::needs_target_archetype`] is true.
    unsafe fn iter_components(
        &mut self,
        source_archetype: &Archetype,
        target_archetype: Option<&Archetype>,
        mut clone_component: impl FnMut(ComponentId),
    ) {
        // clone explicit components
        for (&component, explicit) in self.explicits.iter() {
            let do_clone = match explicit.insert_mode {
                InsertMode::Replace => source_archetype.contains(component),
                InsertMode::Keep => {
                    // SAFETY: caller assured this is `Some`
                    let target_archetype = unsafe { target_archetype.debug_checked_unwrap() };
                    source_archetype.contains(component) && !target_archetype.contains(component)
                }
            };
            if do_clone {
                clone_component(component);
            } else if let Some(range) = explicit.requires.clone() {
                for component in self.requires_of_explicits[range].iter() {
                    let required = self.requires.get_mut(component);
                    // SAFETY: It is certain the ComponentId in `requires_of_explicits` is present in `requires`
                    let required = unsafe { required.debug_checked_unwrap() };
                    // track that this required component should not be cloned solely for this explicit component
                    required.required_by_reduced -= 1;
                }
            }
        }

        // clone required components
        for required_component in self
            .requires
            .iter_mut()
            .filter_map(|(&component, required)| {
                // SAFETY: caller assured this is `Some`
                let target_archetype = unsafe { target_archetype.debug_checked_unwrap() };

                // do not check if source contains component because when `required_by_reduced`
                // did not hit zero at this point, a component that this one here is required
                // by was cloned, so the source must also contain its required components
                let pass = required.required_by_reduced > 0 // required by a cloned component
                    && !target_archetype.contains(component) // do not overwrite existing values
                    && !self.explicits.contains_key(&component); // was not already cloned as explicit

                // revert reductions for the next entity to clone with this EntityCloner
                required.required_by_reduced = required.required_by;

                pass.then_some(component)
            })
        {
            clone_component(required_component);
        }
    }
}

/// A builder for configuring [`EntityCloner`]. See [`EntityCloner`] for more information.
pub struct EntityClonerBuilder<'w, Filter> {
    world: &'w mut World,
    entity_cloner: EntityCloner,
    filter: Filter,
}

impl<'w, Filter> EntityClonerBuilder<'w, Filter> {
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

    /// Overrides the [`ComponentCloneBehavior`] for a component in this builder.
    /// This handler will be used to clone the component instead of the global one defined by the [`EntityCloner`].
    ///
    /// See [Handlers section of `EntityClonerBuilder`](EntityClonerBuilder#handlers) to understand how this affects handler priority.
    pub fn override_clone_behavior<T: Component>(
        &mut self,
        clone_behavior: ComponentCloneBehavior,
    ) -> &mut Self {
        if let Some(id) = self.world.components().valid_component_id::<T>() {
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
        if let Some(id) = self.world.components().valid_component_id::<T>() {
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

    /// When true this cloner will be configured to clone entities referenced in cloned components via [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN).
    /// This will produce "deep" / recursive clones of relationship trees that have "linked spawn".
    pub fn linked_cloning(&mut self, linked_cloning: bool) -> &mut Self {
        self.entity_cloner.linked_cloning = linked_cloning;
        self
    }
}

impl<'w> EntityClonerBuilder<'w, AllowAll> {
    /// Finishes configuring [`EntityCloner`] returns it.
    pub fn finish(mut self) -> EntityCloner {
        self.entity_cloner.filter = Some(AllowOrDenyAll::AllowAll(self.filter));
        self.entity_cloner
    }

    /// Internally calls [`EntityCloner::clone_entity`] on the builder's [`World`].
    pub fn clone_entity(&mut self, source: Entity, target: Entity) -> &mut Self {
        let filter = core::mem::replace(
            &mut self.filter,
            AllowAll {
                deny: Default::default(),
            },
        );
        self.entity_cloner.filter = Some(AllowOrDenyAll::AllowAll(filter));
        self.entity_cloner.clone_entity(self.world, source, target);
        match self.entity_cloner.filter.take() {
            Some(AllowOrDenyAll::AllowAll(filter)) => self.filter = filter,
            _ => unreachable!("todo"),
        }
        self
    }

    /// Disallows all components of the bundle from being cloned.
    pub fn deny<T: Bundle>(&mut self) -> &mut Self {
        let bundle = self.world.register_bundle::<T>();
        let ids = bundle.explicit_components().to_owned();
        for id in ids {
            self.filter.deny.insert(id);
        }
        self
    }

    /// Extends the list of components that shouldn't be cloned.
    pub fn deny_by_ids(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        for id in ids {
            self.filter.deny.insert(id);
        }
        self
    }

    /// Extends the list of components that shouldn't be cloned by type ids.
    pub fn deny_by_type_ids(&mut self, ids: impl IntoIterator<Item = TypeId>) -> &mut Self {
        for type_id in ids {
            if let Some(id) = self.world.components().get_valid_id(type_id) {
                self.filter.deny.insert(id);
            }
        }
        self
    }
}

impl<'w> EntityClonerBuilder<'w, DenyAll> {
    /// Finishes configuring [`EntityCloner`] returns it.
    pub fn finish(mut self) -> EntityCloner {
        self.entity_cloner.filter = Some(AllowOrDenyAll::DenyAll(self.filter));
        self.entity_cloner
    }

    /// Internally calls [`EntityCloner::clone_entity`] on the builder's [`World`].
    pub fn clone_entity(&mut self, source: Entity, target: Entity) -> &mut Self {
        let filter = core::mem::take(&mut self.filter);
        self.entity_cloner.filter = Some(AllowOrDenyAll::DenyAll(filter));
        self.entity_cloner.clone_entity(self.world, source, target);
        match self.entity_cloner.filter.take() {
            Some(AllowOrDenyAll::DenyAll(filter)) => self.filter = filter,
            _ => unreachable!("todo"),
        }
        self
    }

    /// By default, any components allowed/denied through the filter will automatically
    /// allow/deny all of their required components.
    ///
    /// This method allows for a scoped mode where any changes to the filter
    /// will not involve required components.
    pub fn without_required_components(&mut self, builder: impl FnOnce(&mut Self)) -> &mut Self {
        self.filter.attach_required_components = false;
        builder(self);
        self.filter.attach_required_components = true;
        self
    }

    /// Adds all components of the bundle to the list of components to clone.
    pub fn allow<T: Bundle>(&mut self) -> &mut Self {
        let bundle = self.world.register_bundle::<T>();
        let ids = bundle.explicit_components().to_owned();
        for id in ids {
            self.filter
                .filter_allow(id, self.world, InsertMode::Replace);
        }
        self
    }

    /// Adds all components of the bundle to the list of components to clone if the target does not contain them.
    pub fn allow_if_new<T: Bundle>(&mut self) -> &mut Self {
        let bundle = self.world.register_bundle::<T>();
        let ids = bundle.explicit_components().to_owned();
        for id in ids {
            self.filter.filter_allow(id, self.world, InsertMode::Keep);
        }
        self
    }

    /// Adds all components of the bundle ID to the list of components to clone.
    pub fn allow_by_bundle_id(&mut self, bundle_id: BundleId) -> &mut Self {
        if let Some(bundle) = self.world.bundles().get(bundle_id) {
            let ids = bundle.explicit_components().to_owned();
            for id in ids {
                self.filter
                    .filter_allow(id, self.world, InsertMode::Replace);
            }
        }
        self
    }

    /// Adds all components of the bundle ID to the list of components to clone if the target does not contain them.
    pub fn allow_by_bundle_id_if_new(&mut self, bundle_id: BundleId) -> &mut Self {
        if let Some(bundle) = self.world.bundles().get(bundle_id) {
            let ids = bundle.explicit_components().to_owned();
            for id in ids {
                self.filter.filter_allow(id, self.world, InsertMode::Keep);
            }
        }
        self
    }

    /// Extends the list of components to clone.
    pub fn allow_by_ids(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        for id in ids {
            self.filter
                .filter_allow(id, self.world, InsertMode::Replace);
        }
        self
    }

    /// Extends the list of components to clone if the target does not contain them.
    pub fn allow_by_ids_if_new(&mut self, ids: impl IntoIterator<Item = ComponentId>) -> &mut Self {
        for id in ids {
            self.filter.filter_allow(id, self.world, InsertMode::Keep);
        }
        self
    }

    /// Extends the list of components to clone using [`TypeId`]s.
    pub fn allow_by_type_ids(&mut self, ids: impl IntoIterator<Item = TypeId>) -> &mut Self {
        for type_id in ids {
            if let Some(id) = self.world.components().get_valid_id(type_id) {
                self.filter
                    .filter_allow(id, self.world, InsertMode::Replace);
            }
        }
        self
    }

    /// Extends the list of components to clone using [`TypeId`]s if the target does not contain them.
    pub fn allow_by_type_ids_if_new(&mut self, ids: impl IntoIterator<Item = TypeId>) -> &mut Self {
        for type_id in ids {
            if let Some(id) = self.world.components().get_valid_id(type_id) {
                self.filter.filter_allow(id, self.world, InsertMode::Keep);
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::ComponentCloneCtx;
    use crate::{
        component::{Component, ComponentCloneBehavior, ComponentDescriptor, StorageType},
        entity::{Entity, EntityCloner, EntityHashMap, SourceComponent},
        prelude::{ChildOf, Children, Resource},
        reflect::{AppTypeRegistry, ReflectComponent, ReflectFromWorld},
        world::{FromWorld, World},
    };
    use alloc::vec::Vec;
    use bevy_ptr::OwningPtr;
    use bevy_reflect::Reflect;
    use core::marker::PhantomData;
    use core::{alloc::Layout, ops::Deref};

    #[cfg(feature = "bevy_reflect")]
    mod reflect {
        use super::*;
        use crate::{
            component::{Component, ComponentCloneBehavior},
            entity::{EntityCloner, SourceComponent},
            reflect::{AppTypeRegistry, ReflectComponent, ReflectFromWorld},
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

            EntityCloner::build_allow_all(&mut world)
                .override_clone_behavior::<A>(ComponentCloneBehavior::reflect())
                .clone_entity(e, e_clone);

            assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        }

        #[test]
        fn clone_entity_using_reflect_all_paths() {
            #[derive(PartialEq, Eq, Default, Debug)]
            struct NotClone;

            // `reflect_clone`-based fast path
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            #[reflect(from_reflect = false)]
            struct A {
                field: usize,
                field2: Vec<usize>,
            }

            // `ReflectDefault`-based fast path
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            #[reflect(Default)]
            #[reflect(from_reflect = false)]
            struct B {
                field: usize,
                field2: Vec<usize>,
                #[reflect(ignore)]
                ignored: NotClone,
            }

            // `ReflectFromReflect`-based fast path
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            struct C {
                field: usize,
                field2: Vec<usize>,
                #[reflect(ignore)]
                ignored: NotClone,
            }

            // `ReflectFromWorld`-based fast path
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            #[reflect(FromWorld)]
            #[reflect(from_reflect = false)]
            struct D {
                field: usize,
                field2: Vec<usize>,
                #[reflect(ignore)]
                ignored: NotClone,
            }

            let mut world = World::default();
            world.init_resource::<AppTypeRegistry>();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            registry.write().register::<(A, B, C, D)>();

            let a_id = world.register_component::<A>();
            let b_id = world.register_component::<B>();
            let c_id = world.register_component::<C>();
            let d_id = world.register_component::<D>();
            let component_a = A {
                field: 5,
                field2: vec![1, 2, 3, 4, 5],
            };
            let component_b = B {
                field: 5,
                field2: vec![1, 2, 3, 4, 5],
                ignored: NotClone,
            };
            let component_c = C {
                field: 6,
                field2: vec![1, 2, 3, 4, 5],
                ignored: NotClone,
            };
            let component_d = D {
                field: 7,
                field2: vec![1, 2, 3, 4, 5],
                ignored: NotClone,
            };

            let e = world
                .spawn((component_a, component_b, component_c, component_d))
                .id();
            let e_clone = world.spawn_empty().id();

            EntityCloner::build_allow_all(&mut world)
                .override_clone_behavior_with_id(a_id, ComponentCloneBehavior::reflect())
                .override_clone_behavior_with_id(b_id, ComponentCloneBehavior::reflect())
                .override_clone_behavior_with_id(c_id, ComponentCloneBehavior::reflect())
                .override_clone_behavior_with_id(d_id, ComponentCloneBehavior::reflect())
                .clone_entity(e, e_clone);

            assert_eq!(world.get::<A>(e_clone), Some(world.get::<A>(e).unwrap()));
            assert_eq!(world.get::<B>(e_clone), Some(world.get::<B>(e).unwrap()));
            assert_eq!(world.get::<C>(e_clone), Some(world.get::<C>(e).unwrap()));
            assert_eq!(world.get::<D>(e_clone), Some(world.get::<D>(e).unwrap()));
        }

        #[test]
        fn read_source_component_reflect_should_return_none_on_invalid_reflect_from_ptr() {
            #[derive(Component, Reflect)]
            struct A;

            #[derive(Component, Reflect)]
            struct B;

            fn test_handler(source: &SourceComponent, ctx: &mut ComponentCloneCtx) {
                let registry = ctx.type_registry().unwrap();
                assert!(source.read_reflect(&registry.read()).is_none());
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

            EntityCloner::build_allow_all(&mut world)
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

            EntityCloner::build_allow_all(&mut world).clone_entity(e, e_clone);

            assert!(world
                .get::<A>(e_clone)
                .is_some_and(|comp| *comp == A { field: 10 }));
        }

        #[test]
        fn clone_entity_using_reflect_should_skip_without_panic() {
            // Not reflected
            #[derive(Component, PartialEq, Eq, Default, Debug)]
            struct A;

            // No valid type data and not `reflect_clone`-able
            #[derive(Component, Reflect, PartialEq, Eq, Default, Debug)]
            #[reflect(Component)]
            #[reflect(from_reflect = false)]
            struct B(#[reflect(ignore)] PhantomData<()>);

            let mut world = World::default();

            // No AppTypeRegistry
            let e = world.spawn((A, B(Default::default()))).id();
            let e_clone = world.spawn_empty().id();
            EntityCloner::build_allow_all(&mut world)
                .override_clone_behavior::<A>(ComponentCloneBehavior::reflect())
                .override_clone_behavior::<B>(ComponentCloneBehavior::reflect())
                .clone_entity(e, e_clone);
            assert_eq!(world.get::<A>(e_clone), None);
            assert_eq!(world.get::<B>(e_clone), None);

            // With AppTypeRegistry
            world.init_resource::<AppTypeRegistry>();
            let registry = world.get_resource::<AppTypeRegistry>().unwrap();
            registry.write().register::<B>();

            let e = world.spawn((A, B(Default::default()))).id();
            let e_clone = world.spawn_empty().id();
            EntityCloner::build_allow_all(&mut world).clone_entity(e, e_clone);
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

        EntityCloner::build_allow_all(&mut world).clone_entity(e, e_clone);

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

        EntityCloner::build_deny_all(&mut world)
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

        EntityCloner::build_allow_all(&mut world)
            .deny::<B>()
            .clone_entity(e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        assert!(world.get::<B>(e_clone).is_none());
        assert!(world.get::<C>(e_clone).is_some());
    }

    /* todo: API does not support mixing deny/allow anymore, see if this test is still needed or needs to be rewritten
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
    */

    /* todo: API does not support mixing deny/allow anymore, see if this test is still needed or needs to be rewritten
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
    */

    #[test]
    fn clone_entity_with_required_components() {
        #[derive(Component, Clone, PartialEq, Debug)]
        #[require(B)]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(C(5))]
        struct B;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct C(u32);

        let mut world = World::default();

        let e = world.spawn(A).id();
        let e_clone = world.spawn_empty().id();

        EntityCloner::build_deny_all(&mut world)
            .allow::<B>()
            .clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), None);
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B));
        assert_eq!(world.entity(e_clone).get::<C>(), Some(&C(5)));
    }

    #[test]
    fn clone_entity_with_default_required_components() {
        #[derive(Component, Clone, PartialEq, Debug)]
        #[require(B)]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(C(5))]
        struct B;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct C(u32);

        let mut world = World::default();

        let e = world.spawn((A, C(0))).id();
        let e_clone = world.spawn_empty().id();

        EntityCloner::build_deny_all(&mut world)
            .without_required_components(|builder| {
                builder.allow::<A>();
            })
            .clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), Some(&A));
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B));
        assert_eq!(world.entity(e_clone).get::<C>(), Some(&C(5)));
    }

    #[test]
    fn clone_entity_with_dynamic_components() {
        const COMPONENT_SIZE: usize = 10;
        fn test_handler(source: &SourceComponent, ctx: &mut ComponentCloneCtx) {
            // SAFETY: the passed in ptr corresponds to copy-able data that matches the type of the source / target component
            unsafe {
                ctx.write_target_component_ptr(source.ptr());
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
        EntityCloner::build_allow_all(&mut world).clone_entity(entity, entity_clone);

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
        EntityCloner::build_allow_all(&mut world)
            .linked_cloning(true)
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
        struct SomeRef(
            #[entities] Entity,
            // We add an ignored field here to ensure `reflect_clone` fails and `FromWorld` is used
            #[reflect(ignore)] PhantomData<()>,
        );

        #[derive(Resource)]
        struct FromWorldCalled(bool);

        impl FromWorld for SomeRef {
            fn from_world(world: &mut World) -> Self {
                world.insert_resource(FromWorldCalled(true));
                SomeRef(Entity::PLACEHOLDER, Default::default())
            }
        }
        let mut world = World::new();
        let registry = AppTypeRegistry::default();
        registry.write().register::<SomeRef>();
        world.insert_resource(registry);

        let a = world.spawn_empty().id();
        let b = world.spawn_empty().id();
        let c = world.spawn(SomeRef(a, Default::default())).id();
        let d = world.spawn_empty().id();
        let mut map = EntityHashMap::<Entity>::new();
        map.insert(a, b);
        map.insert(c, d);

        let cloned = EntityCloner::default().clone_entity_mapped(&mut world, c, &mut map);
        assert_eq!(
            *world.entity(cloned).get::<SomeRef>().unwrap(),
            SomeRef(b, Default::default())
        );
        assert!(world.resource::<FromWorldCalled>().0);
    }

    #[test]
    fn cloning_with_required_components_preserves_existing() {
        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(B(5))]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct B(u32);

        let mut world = World::default();

        let e = world.spawn((A, B(0))).id();
        let e_clone = world.spawn(B(1)).id();

        EntityCloner::build_deny_all(&mut world)
            .allow::<A>()
            .clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), Some(&A));
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B(1)));
    }
}
