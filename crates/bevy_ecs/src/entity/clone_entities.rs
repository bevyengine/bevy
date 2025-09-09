use alloc::{boxed::Box, collections::VecDeque, vec::Vec};
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_ptr::{Ptr, PtrMut};
use bevy_utils::prelude::DebugName;
use bumpalo::Bump;
use core::{any::TypeId, cell::LazyCell, ops::Range};
use derive_more::derive::From;

use crate::{
    archetype::Archetype,
    bundle::{Bundle, BundleRemover, InsertMode},
    change_detection::MaybeLocation,
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
    target_component_moved: bool,
    bundle_scratch: &'a mut BundleScratch<'b>,
    bundle_scratch_allocator: &'b Bump,
    entities: &'a Entities,
    source: Entity,
    target: Entity,
    component_info: &'a ComponentInfo,
    state: &'a mut EntityClonerState,
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
        entity_cloner: &'a mut EntityClonerState,
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
            target_component_moved: false,
            bundle_scratch_allocator,
            entities,
            mapper,
            component_info,
            state: entity_cloner,
            type_registry,
        }
    }

    /// Returns true if [`write_target_component`](`Self::write_target_component`) was called before.
    pub fn target_component_written(&self) -> bool {
        self.target_component_written
    }

    /// Returns `true` if used in moving context
    pub fn moving(&self) -> bool {
        self.state.move_components
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
        self.state.linked_cloning
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
        let debug_name = DebugName::type_name::<C>();
        let short_name = debug_name.shortname();
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
        self.state.clone_queue.push_back(entity);
    }

    /// Queues a deferred clone operation, which will run with exclusive [`World`] access immediately after calling the clone handler for each component on an entity.
    /// This exists, despite its similarity to [`Commands`](crate::system::Commands), to provide access to the entity mapper in the current context.
    pub fn queue_deferred(
        &mut self,
        deferred: impl FnOnce(&mut World, &mut dyn EntityMapper) + 'static,
    ) {
        self.state.deferred_commands.push_back(Box::new(deferred));
    }

    /// Marks component as moved and it's `drop` won't run.
    fn move_component(&mut self) {
        self.target_component_moved = true;
        self.target_component_written = true;
    }
}

/// A configuration determining how to clone entities. This can be built using [`EntityCloner::build_opt_out`]/
/// [`opt_in`](EntityCloner::build_opt_in), which
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
/// EntityCloner::build_opt_out(&mut world).clone_entity(entity, entity_clone);
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
///
/// # Moving components
/// [`EntityCloner`] can be configured to move components instead of cloning them by using [`EntityClonerBuilder::move_components`].
/// In this mode components will be moved - removed from source entity and added to the target entity.
///
/// Components with [`ComponentCloneBehavior::Ignore`] clone behavior will not be moved, while components that
/// have a [`ComponentCloneBehavior::Custom`] clone behavior will be cloned using it and then removed from the source entity.
/// All other components will be bitwise copied from the source entity onto the target entity and then removed without dropping.
///
/// Choosing to move components instead of cloning makes [`EntityClonerBuilder::with_default_clone_fn`] ineffective since it's replaced by
/// move handler for components that have [`ComponentCloneBehavior::Default`] clone behavior.
///
/// Note that moving components still triggers `on_remove` hooks/observers on source entity and `on_insert`/`on_add` hooks/observers on the target entity.
#[derive(Default)]
pub struct EntityCloner {
    filter: EntityClonerFilter,
    state: EntityClonerState,
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
    #[track_caller]
    pub(crate) unsafe fn write(
        self,
        world: &mut World,
        entity: Entity,
        relationship_hook_insert_mode: RelationshipHookMode,
    ) {
        // SAFETY:
        // - All `component_ids` are from the same world as `entity`
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
    /// Returns a new [`EntityClonerBuilder`] using the given `world` with the [`OptOut`] configuration.
    ///
    /// This builder tries to clone every component from the source entity except for components that were
    /// explicitly denied, for example by using the [`deny`](EntityClonerBuilder<OptOut>::deny) method.
    ///
    /// Required components are not considered by denied components and must be explicitly denied as well if desired.
    pub fn build_opt_out(world: &mut World) -> EntityClonerBuilder<'_, OptOut> {
        EntityClonerBuilder {
            world,
            filter: Default::default(),
            state: Default::default(),
        }
    }

    /// Returns a new [`EntityClonerBuilder`] using the given `world` with the [`OptIn`] configuration.
    ///
    /// This builder tries to clone every component that was explicitly allowed from the source entity,
    /// for example by using the [`allow`](EntityClonerBuilder<OptIn>::allow) method.
    ///
    /// Components allowed to be cloned through this builder would also allow their required components,
    /// which will be cloned from the source entity only if the target entity does not contain them already.
    /// To skip adding required components see [`without_required_components`](EntityClonerBuilder<OptIn>::without_required_components).
    pub fn build_opt_in(world: &mut World) -> EntityClonerBuilder<'_, OptIn> {
        EntityClonerBuilder {
            world,
            filter: Default::default(),
            state: Default::default(),
        }
    }

    /// Returns `true` if this cloner is configured to clone entities referenced in cloned components via [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN).
    /// This will produce "deep" / recursive clones of relationship trees that have "linked spawn".
    #[inline]
    pub fn linked_cloning(&self) -> bool {
        self.state.linked_cloning
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
        Self::clone_entity_mapped_internal(&mut self.state, &mut self.filter, world, source, mapper)
    }

    #[track_caller]
    #[inline]
    fn clone_entity_mapped_internal(
        state: &mut EntityClonerState,
        filter: &mut impl CloneByFilter,
        world: &mut World,
        source: Entity,
        mapper: &mut dyn EntityMapper,
    ) -> Entity {
        // All relationships on the root should have their hooks run
        let target = Self::clone_entity_internal(
            state,
            filter,
            world,
            source,
            mapper,
            RelationshipHookMode::Run,
        );
        let child_hook_insert_mode = if state.linked_cloning {
            // When spawning "linked relationships", we want to ignore hooks for relationships we are spawning, while
            // still registering with original relationship targets that are "not linked" to the current recursive spawn.
            RelationshipHookMode::RunIfNotLinked
        } else {
            // If we are not cloning "linked relationships" recursively, then we want any cloned relationship components to
            // register themselves with their original relationship target.
            RelationshipHookMode::Run
        };
        loop {
            let queued = state.clone_queue.pop_front();
            if let Some(queued) = queued {
                Self::clone_entity_internal(
                    state,
                    filter,
                    world,
                    queued,
                    mapper,
                    child_hook_insert_mode,
                );
            } else {
                break;
            }
        }
        target
    }

    /// Clones and inserts components from the `source` entity into the entity mapped by `mapper` from `source` using the stored configuration.
    #[track_caller]
    fn clone_entity_internal(
        state: &mut EntityClonerState,
        filter: &mut impl CloneByFilter,
        world: &mut World,
        source: Entity,
        mapper: &mut dyn EntityMapper,
        relationship_hook_insert_mode: RelationshipHookMode,
    ) -> Entity {
        let target = mapper.get_mapped(source);
        // PERF: reusing allocated space across clones would be more efficient. Consider an allocation model similar to `Commands`.
        let bundle_scratch_allocator = Bump::new();
        let mut bundle_scratch: BundleScratch;
        let mut moved_components: Vec<ComponentId> = Vec::new();
        let mut deferred_cloned_component_ids: Vec<ComponentId> = Vec::new();
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

            let source_archetype = source_entity.archetype();
            bundle_scratch = BundleScratch::with_capacity(source_archetype.component_count());

            let target_archetype = LazyCell::new(|| {
                world
                    .get_entity(target)
                    .expect("Target entity must exist")
                    .archetype()
            });

            if state.move_components {
                moved_components.reserve(source_archetype.component_count());
                // Replace default handler with special handler which would track if component was moved instead of cloned.
                // This is later used to determine whether we need to run component's drop function when removing it from the source entity or not.
                state.default_clone_fn = |_, ctx| ctx.move_component();
            }

            filter.clone_components(source_archetype, target_archetype, |component| {
                let handler = match state.clone_behavior_overrides.get(&component).or_else(|| {
                    world
                        .components()
                        .get_info(component)
                        .map(ComponentInfo::clone_behavior)
                }) {
                    Some(behavior) => match behavior {
                        ComponentCloneBehavior::Default => state.default_clone_fn,
                        ComponentCloneBehavior::Ignore => return,
                        ComponentCloneBehavior::Custom(custom) => *custom,
                    },
                    None => state.default_clone_fn,
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
                        state,
                        mapper,
                        app_registry.as_ref(),
                    )
                };

                (handler)(&source_component, &mut ctx);

                if ctx.state.move_components {
                    if ctx.target_component_moved {
                        moved_components.push(component);
                    }
                    // Component wasn't written by the clone handler, so assume it's going to be
                    // cloned/processed using deferred_commands instead.
                    // This means that it's ComponentId won't be present in BundleScratch's component_ids,
                    // but it should still be removed when move_components is true.
                    else if !ctx.target_component_written() {
                        deferred_cloned_component_ids.push(component);
                    }
                }
            });
        }

        world.flush();

        for deferred in state.deferred_commands.drain(..) {
            (deferred)(world, mapper);
        }

        if !world.entities.contains(target) {
            panic!("Target entity does not exist");
        }

        if state.move_components {
            let mut source_entity = world.entity_mut(source);

            let cloned_components = if deferred_cloned_component_ids.is_empty() {
                &bundle_scratch.component_ids
            } else {
                // Remove all cloned components with drop by concatenating both vectors
                deferred_cloned_component_ids.extend(&bundle_scratch.component_ids);
                &deferred_cloned_component_ids
            };
            source_entity.remove_by_ids_with_caller(
                cloned_components,
                MaybeLocation::caller(),
                RelationshipHookMode::RunIfNotLinked,
                BundleRemover::empty_pre_remove,
            );

            let table_row = source_entity.location().table_row;

            // Copy moved components and then forget them without calling drop
            source_entity.remove_by_ids_with_caller(
                &moved_components,
                MaybeLocation::caller(),
                RelationshipHookMode::RunIfNotLinked,
                |sparse_sets, mut table, components, bundle| {
                    for &component_id in bundle {
                        let Some(component_ptr) = sparse_sets
                            .get(component_id)
                            .and_then(|component| component.get(source))
                            .or_else(|| {
                                // SAFETY: table_row is within this table because we just got it from entity's current location
                                table.as_mut().and_then(|table| unsafe {
                                    table.get_component(component_id, table_row)
                                })
                            })
                        else {
                            // Component was removed by some other component's clone side effect before we got to it.
                            continue;
                        };

                        // SAFETY: component_id is valid because remove_by_ids_with_caller checked it before calling this closure
                        let info = unsafe { components.get_info_unchecked(component_id) };
                        let layout = info.layout();
                        let target_ptr = bundle_scratch_allocator.alloc_layout(layout);
                        // SAFETY:
                        // - component_ptr points to data with component layout
                        // - target_ptr was just allocated with component layout
                        // - component_ptr and target_ptr don't overlap
                        // - component_ptr matches component_id
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                component_ptr.as_ptr(),
                                target_ptr.as_ptr(),
                                layout.size(),
                            );
                            bundle_scratch.push_ptr(component_id, PtrMut::new(target_ptr));
                        }
                    }

                    (/* should drop? */ false, ())
                },
            );
        }

        // SAFETY:
        // - All `component_ids` are from the same world as `target` entity
        // - All `component_data_ptrs` are valid types represented by `component_ids`
        unsafe { bundle_scratch.write(world, target, relationship_hook_insert_mode) };
        target
    }
}

/// Part of the [`EntityCloner`], see there for more information.
struct EntityClonerState {
    clone_behavior_overrides: HashMap<ComponentId, ComponentCloneBehavior>,
    move_components: bool,
    linked_cloning: bool,
    default_clone_fn: ComponentCloneFn,
    clone_queue: VecDeque<Entity>,
    deferred_commands: VecDeque<Box<dyn FnOnce(&mut World, &mut dyn EntityMapper)>>,
}

impl Default for EntityClonerState {
    fn default() -> Self {
        Self {
            move_components: false,
            linked_cloning: false,
            default_clone_fn: ComponentCloneBehavior::global_default_fn(),
            clone_behavior_overrides: Default::default(),
            clone_queue: Default::default(),
            deferred_commands: Default::default(),
        }
    }
}

/// A builder for configuring [`EntityCloner`]. See [`EntityCloner`] for more information.
pub struct EntityClonerBuilder<'w, Filter> {
    world: &'w mut World,
    filter: Filter,
    state: EntityClonerState,
}

impl<'w, Filter: CloneByFilter> EntityClonerBuilder<'w, Filter> {
    /// Internally calls [`EntityCloner::clone_entity`] on the builder's [`World`].
    pub fn clone_entity(&mut self, source: Entity, target: Entity) -> &mut Self {
        let mut mapper = EntityHashMap::<Entity>::new();
        mapper.set_mapped(source, target);
        EntityCloner::clone_entity_mapped_internal(
            &mut self.state,
            &mut self.filter,
            self.world,
            source,
            &mut mapper,
        );
        self
    }

    /// Finishes configuring [`EntityCloner`] returns it.
    pub fn finish(self) -> EntityCloner {
        EntityCloner {
            filter: self.filter.into(),
            state: self.state,
        }
    }

    /// Sets the default clone function to use.
    ///
    /// Will be overridden if [`EntityClonerBuilder::move_components`] is enabled.
    pub fn with_default_clone_fn(&mut self, clone_fn: ComponentCloneFn) -> &mut Self {
        self.state.default_clone_fn = clone_fn;
        self
    }

    /// Sets whether the cloner should remove any components that were cloned,
    /// effectively moving them from the source entity to the target.
    ///
    /// This is disabled by default.
    ///
    /// The setting only applies to components that are allowed through the filter
    /// at the time [`EntityClonerBuilder::clone_entity`] is called.
    ///
    /// Enabling this overrides any custom function set with [`EntityClonerBuilder::with_default_clone_fn`].
    pub fn move_components(&mut self, enable: bool) -> &mut Self {
        self.state.move_components = enable;
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
            self.state
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
        self.state
            .clone_behavior_overrides
            .insert(component_id, clone_behavior);
        self
    }

    /// Removes a previously set override of [`ComponentCloneBehavior`] for a component in this builder.
    pub fn remove_clone_behavior_override<T: Component>(&mut self) -> &mut Self {
        if let Some(id) = self.world.components().valid_component_id::<T>() {
            self.state.clone_behavior_overrides.remove(&id);
        }
        self
    }

    /// Removes a previously set override of [`ComponentCloneBehavior`] for a given `component_id` in this builder.
    pub fn remove_clone_behavior_override_with_id(
        &mut self,
        component_id: ComponentId,
    ) -> &mut Self {
        self.state.clone_behavior_overrides.remove(&component_id);
        self
    }

    /// When true this cloner will be configured to clone entities referenced in cloned components via [`RelationshipTarget::LINKED_SPAWN`](crate::relationship::RelationshipTarget::LINKED_SPAWN).
    /// This will produce "deep" / recursive clones of relationship trees that have "linked spawn".
    pub fn linked_cloning(&mut self, linked_cloning: bool) -> &mut Self {
        self.state.linked_cloning = linked_cloning;
        self
    }
}

impl<'w> EntityClonerBuilder<'w, OptOut> {
    /// By default, any components denied through the filter will automatically
    /// deny all of components they are required by too.
    ///
    /// This method allows for a scoped mode where any changes to the filter
    /// will not involve these requiring components.
    ///
    /// If component `A` is denied in the `builder` closure here and component `B`
    /// requires `A`, then `A` will be inserted with the value defined in `B`'s
    /// [`Component` derive](https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html#required-components).
    /// This assumes `A` is missing yet at the target entity.
    pub fn without_required_by_components(&mut self, builder: impl FnOnce(&mut Self)) -> &mut Self {
        self.filter.attach_required_by_components = false;
        builder(self);
        self.filter.attach_required_by_components = true;
        self
    }

    /// Sets whether components are always cloned ([`InsertMode::Replace`], the default) or only if it is missing
    /// ([`InsertMode::Keep`]) at the target entity.
    ///
    /// This makes no difference if the target is spawned by the cloner.
    pub fn insert_mode(&mut self, insert_mode: InsertMode) -> &mut Self {
        self.filter.insert_mode = insert_mode;
        self
    }

    /// Disallows all components of the bundle from being cloned.
    ///
    /// If component `A` is denied here and component `B` requires `A`, then `A`
    /// is denied as well. See [`Self::without_required_by_components`] to alter
    /// this behavior.
    pub fn deny<T: Bundle>(&mut self) -> &mut Self {
        let bundle_id = self.world.register_bundle::<T>().id();
        self.deny_by_ids(bundle_id)
    }

    /// Extends the list of components that shouldn't be cloned.
    /// Supports filtering by [`TypeId`], [`ComponentId`], [`BundleId`](`crate::bundle::BundleId`), and [`IntoIterator`] yielding one of these.
    ///
    /// If component `A` is denied here and component `B` requires `A`, then `A`
    /// is denied as well. See [`Self::without_required_by_components`] to alter
    /// this behavior.
    pub fn deny_by_ids<M: Marker>(&mut self, ids: impl FilterableIds<M>) -> &mut Self {
        ids.filter_ids(&mut |ids| match ids {
            FilterableId::Type(type_id) => {
                if let Some(id) = self.world.components().get_valid_id(type_id) {
                    self.filter.filter_deny(id, self.world);
                }
            }
            FilterableId::Component(component_id) => {
                self.filter.filter_deny(component_id, self.world);
            }
            FilterableId::Bundle(bundle_id) => {
                if let Some(bundle) = self.world.bundles().get(bundle_id) {
                    let ids = bundle.explicit_components().iter();
                    for &id in ids {
                        self.filter.filter_deny(id, self.world);
                    }
                }
            }
        });
        self
    }
}

impl<'w> EntityClonerBuilder<'w, OptIn> {
    /// By default, any components allowed through the filter will automatically
    /// allow all of their required components.
    ///
    /// This method allows for a scoped mode where any changes to the filter
    /// will not involve required components.
    ///
    /// If component `A` is allowed in the `builder` closure here and requires
    /// component `B`, then `B` will be inserted with the value defined in `A`'s
    /// [`Component` derive](https://docs.rs/bevy/latest/bevy/ecs/component/trait.Component.html#required-components).
    /// This assumes `B` is missing yet at the target entity.
    pub fn without_required_components(&mut self, builder: impl FnOnce(&mut Self)) -> &mut Self {
        self.filter.attach_required_components = false;
        builder(self);
        self.filter.attach_required_components = true;
        self
    }

    /// Adds all components of the bundle to the list of components to clone.
    ///
    /// If component `A` is allowed here and requires component `B`, then `B`
    /// is allowed as well. See [`Self::without_required_components`]
    /// to alter this behavior.
    pub fn allow<T: Bundle>(&mut self) -> &mut Self {
        let bundle_id = self.world.register_bundle::<T>().id();
        self.allow_by_ids(bundle_id)
    }

    /// Adds all components of the bundle to the list of components to clone if
    /// the target does not contain them.
    ///
    /// If component `A` is allowed here and requires component `B`, then `B`
    /// is allowed as well. See [`Self::without_required_components`]
    /// to alter this behavior.
    pub fn allow_if_new<T: Bundle>(&mut self) -> &mut Self {
        let bundle_id = self.world.register_bundle::<T>().id();
        self.allow_by_ids_if_new(bundle_id)
    }

    /// Extends the list of components to clone.
    /// Supports filtering by [`TypeId`], [`ComponentId`], [`BundleId`](`crate::bundle::BundleId`), and [`IntoIterator`] yielding one of these.
    ///
    /// If component `A` is allowed here and requires component `B`, then `B`
    /// is allowed as well. See [`Self::without_required_components`]
    /// to alter this behavior.
    pub fn allow_by_ids<M: Marker>(&mut self, ids: impl FilterableIds<M>) -> &mut Self {
        self.allow_by_ids_inner(ids, InsertMode::Replace);
        self
    }

    /// Extends the list of components to clone if the target does not contain them.
    /// Supports filtering by [`TypeId`], [`ComponentId`], [`BundleId`](`crate::bundle::BundleId`), and [`IntoIterator`] yielding one of these.
    ///
    /// If component `A` is allowed here and requires component `B`, then `B`
    /// is allowed as well. See [`Self::without_required_components`]
    /// to alter this behavior.
    pub fn allow_by_ids_if_new<M: Marker>(&mut self, ids: impl FilterableIds<M>) -> &mut Self {
        self.allow_by_ids_inner(ids, InsertMode::Keep);
        self
    }

    fn allow_by_ids_inner<M: Marker>(
        &mut self,
        ids: impl FilterableIds<M>,
        insert_mode: InsertMode,
    ) {
        ids.filter_ids(&mut |id| match id {
            FilterableId::Type(type_id) => {
                if let Some(id) = self.world.components().get_valid_id(type_id) {
                    self.filter.filter_allow(id, self.world, insert_mode);
                }
            }
            FilterableId::Component(component_id) => {
                self.filter
                    .filter_allow(component_id, self.world, insert_mode);
            }
            FilterableId::Bundle(bundle_id) => {
                if let Some(bundle) = self.world.bundles().get(bundle_id) {
                    let ids = bundle.explicit_components().iter();
                    for &id in ids {
                        self.filter.filter_allow(id, self.world, insert_mode);
                    }
                }
            }
        });
    }
}

/// Filters that can selectively clone components depending on its inner configuration are unified with this trait.
#[doc(hidden)]
pub trait CloneByFilter: Into<EntityClonerFilter> {
    /// The filter will call `clone_component` for every [`ComponentId`] that passes it.
    fn clone_components<'a>(
        &mut self,
        source_archetype: &Archetype,
        target_archetype: LazyCell<&'a Archetype, impl FnOnce() -> &'a Archetype>,
        clone_component: impl FnMut(ComponentId),
    );
}

/// Part of the [`EntityCloner`], see there for more information.
#[doc(hidden)]
#[derive(From)]
pub enum EntityClonerFilter {
    OptOut(OptOut),
    OptIn(OptIn),
}

impl Default for EntityClonerFilter {
    fn default() -> Self {
        Self::OptOut(Default::default())
    }
}

impl CloneByFilter for EntityClonerFilter {
    #[inline]
    fn clone_components<'a>(
        &mut self,
        source_archetype: &Archetype,
        target_archetype: LazyCell<&'a Archetype, impl FnOnce() -> &'a Archetype>,
        clone_component: impl FnMut(ComponentId),
    ) {
        match self {
            Self::OptOut(filter) => {
                filter.clone_components(source_archetype, target_archetype, clone_component);
            }
            Self::OptIn(filter) => {
                filter.clone_components(source_archetype, target_archetype, clone_component);
            }
        }
    }
}

/// Generic for [`EntityClonerBuilder`] that makes the cloner try to clone every component from the source entity
/// except for components that were explicitly denied, for example by using the
/// [`deny`](EntityClonerBuilder::deny) method.
///
/// Required components are not considered by denied components and must be explicitly denied as well if desired.
pub struct OptOut {
    /// Contains the components that should not be cloned.
    deny: HashSet<ComponentId>,

    /// Determines if a component is inserted when it is existing already.
    insert_mode: InsertMode,

    /// Is `true` unless during [`EntityClonerBuilder::without_required_by_components`] which will suppress
    /// components that require denied components to be denied as well, causing them to be created independent
    /// from the value at the source entity if needed.
    attach_required_by_components: bool,
}

impl Default for OptOut {
    fn default() -> Self {
        Self {
            deny: Default::default(),
            insert_mode: InsertMode::Replace,
            attach_required_by_components: true,
        }
    }
}

impl CloneByFilter for OptOut {
    #[inline]
    fn clone_components<'a>(
        &mut self,
        source_archetype: &Archetype,
        target_archetype: LazyCell<&'a Archetype, impl FnOnce() -> &'a Archetype>,
        mut clone_component: impl FnMut(ComponentId),
    ) {
        match self.insert_mode {
            InsertMode::Replace => {
                for component in source_archetype.components() {
                    if !self.deny.contains(&component) {
                        clone_component(component);
                    }
                }
            }
            InsertMode::Keep => {
                for component in source_archetype.components() {
                    if !target_archetype.contains(component) && !self.deny.contains(&component) {
                        clone_component(component);
                    }
                }
            }
        }
    }
}

impl OptOut {
    /// Denies a component through the filter, also deny components that require `id` if
    /// [`Self::attach_required_by_components`] is true.
    #[inline]
    fn filter_deny(&mut self, id: ComponentId, world: &World) {
        self.deny.insert(id);
        if self.attach_required_by_components {
            if let Some(required_by) = world.components().get_required_by(id) {
                self.deny.extend(required_by.iter());
            };
        }
    }
}

/// Generic for [`EntityClonerBuilder`] that makes the cloner try to clone every component that was explicitly
/// allowed from the source entity, for example by using the [`allow`](EntityClonerBuilder::allow) method.
///
/// Required components are also cloned when the target entity does not contain them.
pub struct OptIn {
    /// Contains the components explicitly allowed to be cloned.
    allow: HashMap<ComponentId, Explicit>,

    /// Lists of required components, [`Explicit`] refers to a range in it.
    required_of_allow: Vec<ComponentId>,

    /// Contains the components required by those in [`Self::allow`].
    /// Also contains the number of components in [`Self::allow`] each is required by to track
    /// when to skip cloning a required component after skipping explicit components that require it.
    required: HashMap<ComponentId, Required>,

    /// Is `true` unless during [`EntityClonerBuilder::without_required_components`] which will suppress
    /// evaluating required components to clone, causing them to be created independent from the value at
    /// the source entity if needed.
    attach_required_components: bool,
}

impl Default for OptIn {
    fn default() -> Self {
        Self {
            allow: Default::default(),
            required_of_allow: Default::default(),
            required: Default::default(),
            attach_required_components: true,
        }
    }
}

impl CloneByFilter for OptIn {
    #[inline]
    fn clone_components<'a>(
        &mut self,
        source_archetype: &Archetype,
        target_archetype: LazyCell<&'a Archetype, impl FnOnce() -> &'a Archetype>,
        mut clone_component: impl FnMut(ComponentId),
    ) {
        // track the amount of components left not being cloned yet to exit this method early
        let mut uncloned_components = source_archetype.component_count();

        // track if any `Required::required_by_reduced` has been reduced so they are reset
        let mut reduced_any = false;

        // clone explicit components
        for (&component, explicit) in self.allow.iter() {
            if uncloned_components == 0 {
                // exhausted all source components, reset changed `Required::required_by_reduced`
                if reduced_any {
                    self.required
                        .iter_mut()
                        .for_each(|(_, required)| required.reset());
                }
                return;
            }

            let do_clone = source_archetype.contains(component)
                && (explicit.insert_mode == InsertMode::Replace
                    || !target_archetype.contains(component));
            if do_clone {
                clone_component(component);
                uncloned_components -= 1;
            } else if let Some(range) = explicit.required_range.clone() {
                for component in self.required_of_allow[range].iter() {
                    // may be None if required component was also added as explicit later
                    if let Some(required) = self.required.get_mut(component) {
                        required.required_by_reduced -= 1;
                        reduced_any = true;
                    }
                }
            }
        }

        let mut required_iter = self.required.iter_mut();

        // clone required components
        let required_components = required_iter
            .by_ref()
            .filter_map(|(&component, required)| {
                let do_clone = required.required_by_reduced > 0 // required by a cloned component
                    && source_archetype.contains(component) // must exist to clone, may miss if removed
                    && !target_archetype.contains(component); // do not overwrite existing values

                // reset changed `Required::required_by_reduced` as this is done being checked here
                required.reset();

                do_clone.then_some(component)
            })
            .take(uncloned_components);

        for required_component in required_components {
            clone_component(required_component);
        }

        // if the `required_components` iterator has not been exhausted yet because the source has no more
        // components to clone, iterate the rest to reset changed `Required::required_by_reduced` for the
        // next clone
        if reduced_any {
            required_iter.for_each(|(_, required)| required.reset());
        }
    }
}

impl OptIn {
    /// Allows a component through the filter, also allow required components if
    /// [`Self::attach_required_components`] is true.
    #[inline]
    fn filter_allow(&mut self, id: ComponentId, world: &World, mut insert_mode: InsertMode) {
        match self.allow.entry(id) {
            Entry::Vacant(explicit) => {
                // explicit components should not appear in the required map
                self.required.remove(&id);

                if !self.attach_required_components {
                    explicit.insert(Explicit {
                        insert_mode,
                        required_range: None,
                    });
                } else {
                    self.filter_allow_with_required(id, world, insert_mode);
                }
            }
            Entry::Occupied(mut explicit) => {
                let explicit = explicit.get_mut();

                // set required component range if it was inserted with `None` earlier
                if self.attach_required_components && explicit.required_range.is_none() {
                    if explicit.insert_mode == InsertMode::Replace {
                        // do not overwrite with Keep if component was allowed as Replace earlier
                        insert_mode = InsertMode::Replace;
                    }

                    self.filter_allow_with_required(id, world, insert_mode);
                } else if explicit.insert_mode == InsertMode::Keep {
                    // potentially overwrite Keep with Replace
                    explicit.insert_mode = insert_mode;
                }
            }
        };
    }

    // Allow a component through the filter and include required components.
    #[inline]
    fn filter_allow_with_required(
        &mut self,
        id: ComponentId,
        world: &World,
        insert_mode: InsertMode,
    ) {
        let Some(info) = world.components().get_info(id) else {
            return;
        };

        let iter = info
            .required_components()
            .iter_ids()
            .filter(|id| !self.allow.contains_key(id))
            .inspect(|id| {
                // set or increase the number of components this `id` is required by
                self.required
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

        let start = self.required_of_allow.len();
        self.required_of_allow.extend(iter);
        let end = self.required_of_allow.len();

        self.allow.insert(
            id,
            Explicit {
                insert_mode,
                required_range: Some(start..end),
            },
        );
    }
}

/// Contains the components explicitly allowed to be cloned.
struct Explicit {
    /// If component was added via [`allow`](EntityClonerBuilder::allow) etc, this is `Overwrite`.
    ///
    /// If component was added via [`allow_if_new`](EntityClonerBuilder::allow_if_new) etc, this is `Keep`.
    insert_mode: InsertMode,

    /// Contains the range in [`OptIn::required_of_allow`] for this component containing its
    /// required components.
    ///
    /// Is `None` if [`OptIn::attach_required_components`] was `false` when added.
    /// It may be set to `Some` later if the component is later added explicitly again with
    /// [`OptIn::attach_required_components`] being `true`.
    ///
    /// Range is empty if this component has no required components that are not also explicitly allowed.
    required_range: Option<Range<usize>>,
}

struct Required {
    /// Amount of explicit components this component is required by.
    required_by: u32,

    /// As [`Self::required_by`] but is reduced during cloning when an explicit component is not cloned,
    /// either because [`Explicit::insert_mode`] is `Keep` or the source entity does not contain it.
    ///
    /// If this is zero, the required component is not cloned.
    ///
    /// The counter is reset to `required_by` when the cloning is over in case another entity needs to be
    /// cloned by the same [`EntityCloner`].
    required_by_reduced: u32,
}

impl Required {
    // Revert reductions for the next entity to clone with this EntityCloner
    #[inline]
    fn reset(&mut self) {
        self.required_by_reduced = self.required_by;
    }
}

mod private {
    use crate::{bundle::BundleId, component::ComponentId};
    use core::any::TypeId;
    use derive_more::From;

    /// Marker trait to allow multiple blanket implementations for [`FilterableIds`].
    pub trait Marker {}
    /// Marker struct for [`FilterableIds`] implementation for single-value types.
    pub struct ScalarType {}
    impl Marker for ScalarType {}
    /// Marker struct for [`FilterableIds`] implementation for [`IntoIterator`] types.
    pub struct VectorType {}
    impl Marker for VectorType {}

    /// Defines types of ids that [`EntityClonerBuilder`](`super::EntityClonerBuilder`) can filter components by.
    #[derive(From)]
    pub enum FilterableId {
        Type(TypeId),
        Component(ComponentId),
        Bundle(BundleId),
    }

    impl<'a, T> From<&'a T> for FilterableId
    where
        T: Into<FilterableId> + Copy,
    {
        #[inline]
        fn from(value: &'a T) -> Self {
            (*value).into()
        }
    }

    /// A trait to allow [`EntityClonerBuilder`](`super::EntityClonerBuilder`) filter by any supported id type and their iterators,
    /// reducing the number of method permutations required for all id types.
    ///
    /// The supported id types that can be used to filter components are defined by [`FilterableId`], which allows following types: [`TypeId`], [`ComponentId`] and [`BundleId`].
    ///
    /// `M` is a generic marker to allow multiple blanket implementations of this trait.
    /// This works because `FilterableId<M1>` is a different trait from `FilterableId<M2>`, so multiple blanket implementations for different `M` are allowed.
    /// The reason this is required is because supporting `IntoIterator` requires blanket implementation, but that will conflict with implementation for `TypeId`
    /// since `IntoIterator` can technically be implemented for `TypeId` in the future.
    /// Functions like `allow_by_ids` rely on type inference to automatically select proper type for `M` at call site.
    pub trait FilterableIds<M: Marker> {
        /// Takes in a function that processes all types of [`FilterableId`] one-by-one.
        fn filter_ids(self, ids: &mut impl FnMut(FilterableId));
    }

    impl<I, T> FilterableIds<VectorType> for I
    where
        I: IntoIterator<Item = T>,
        T: Into<FilterableId>,
    {
        #[inline]
        fn filter_ids(self, ids: &mut impl FnMut(FilterableId)) {
            for id in self.into_iter() {
                ids(id.into());
            }
        }
    }

    impl<T> FilterableIds<ScalarType> for T
    where
        T: Into<FilterableId>,
    {
        #[inline]
        fn filter_ids(self, ids: &mut impl FnMut(FilterableId)) {
            ids(self.into());
        }
    }
}

use private::{FilterableId, FilterableIds, Marker};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        component::{ComponentDescriptor, StorageType},
        lifecycle::HookContext,
        prelude::{ChildOf, Children, Resource},
        world::{DeferredWorld, FromWorld, World},
    };
    use bevy_ptr::OwningPtr;
    use core::marker::PhantomData;
    use core::{alloc::Layout, ops::Deref};

    #[cfg(feature = "bevy_reflect")]
    mod reflect {
        use super::*;
        use crate::reflect::{AppTypeRegistry, ReflectComponent, ReflectFromWorld};
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

            EntityCloner::build_opt_out(&mut world)
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

            EntityCloner::build_opt_out(&mut world)
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
                    .get_mut(TypeId::of::<A>())
                    .unwrap()
                    .insert(<ReflectFromPtr as FromType<B>>::from_type());
            }

            let e = world.spawn(A).id();
            let e_clone = world.spawn_empty().id();

            EntityCloner::build_opt_out(&mut world)
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

            EntityCloner::build_opt_out(&mut world).clone_entity(e, e_clone);

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
            EntityCloner::build_opt_out(&mut world)
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
            EntityCloner::build_opt_out(&mut world).clone_entity(e, e_clone);
            assert_eq!(world.get::<A>(e_clone), None);
            assert_eq!(world.get::<B>(e_clone), None);
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

        EntityCloner::build_opt_out(&mut world).clone_entity(e, e_clone);

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

        EntityCloner::build_opt_in(&mut world)
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
        #[require(C)]
        struct B;

        #[derive(Component, Clone, Default)]
        struct C;

        let mut world = World::default();

        let component = A { field: 5 };

        let e = world.spawn((component.clone(), B, C)).id();
        let e_clone = world.spawn_empty().id();

        EntityCloner::build_opt_out(&mut world)
            .deny::<C>()
            .clone_entity(e, e_clone);

        assert!(world.get::<A>(e_clone).is_some_and(|c| *c == component));
        assert!(world.get::<B>(e_clone).is_none());
        assert!(world.get::<C>(e_clone).is_none());
    }

    #[test]
    fn clone_entity_with_deny_filter_without_required_by() {
        #[derive(Component, Clone)]
        #[require(B { field: 5 })]
        struct A;

        #[derive(Component, Clone, PartialEq, Eq)]
        struct B {
            field: usize,
        }

        let mut world = World::default();

        let e = world.spawn((A, B { field: 10 })).id();
        let e_clone = world.spawn_empty().id();

        EntityCloner::build_opt_out(&mut world)
            .without_required_by_components(|builder| {
                builder.deny::<B>();
            })
            .clone_entity(e, e_clone);

        assert!(world.get::<A>(e_clone).is_some());
        assert!(world
            .get::<B>(e_clone)
            .is_some_and(|c| *c == B { field: 5 }));
    }

    #[test]
    fn clone_entity_with_deny_filter_if_new() {
        #[derive(Component, Clone, PartialEq, Eq)]
        struct A {
            field: usize,
        }

        #[derive(Component, Clone)]
        struct B;

        #[derive(Component, Clone)]
        struct C;

        let mut world = World::default();

        let e = world.spawn((A { field: 5 }, B, C)).id();
        let e_clone = world.spawn(A { field: 8 }).id();

        EntityCloner::build_opt_out(&mut world)
            .deny::<B>()
            .insert_mode(InsertMode::Keep)
            .clone_entity(e, e_clone);

        assert!(world
            .get::<A>(e_clone)
            .is_some_and(|c| *c == A { field: 8 }));
        assert!(world.get::<B>(e_clone).is_none());
        assert!(world.get::<C>(e_clone).is_some());
    }

    #[test]
    fn allow_and_allow_if_new_always_allows() {
        #[derive(Component, Clone, PartialEq, Debug)]
        struct A(u8);

        let mut world = World::default();
        let e = world.spawn(A(1)).id();
        let e_clone1 = world.spawn(A(2)).id();

        EntityCloner::build_opt_in(&mut world)
            .allow_if_new::<A>()
            .allow::<A>()
            .clone_entity(e, e_clone1);

        assert_eq!(world.get::<A>(e_clone1), Some(&A(1)));

        let e_clone2 = world.spawn(A(2)).id();

        EntityCloner::build_opt_in(&mut world)
            .allow::<A>()
            .allow_if_new::<A>()
            .clone_entity(e, e_clone2);

        assert_eq!(world.get::<A>(e_clone2), Some(&A(1)));
    }

    #[test]
    fn with_and_without_required_components_include_required() {
        #[derive(Component, Clone, PartialEq, Debug)]
        #[require(B(5))]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct B(u8);

        let mut world = World::default();
        let e = world.spawn((A, B(10))).id();
        let e_clone1 = world.spawn_empty().id();
        EntityCloner::build_opt_in(&mut world)
            .without_required_components(|builder| {
                builder.allow::<A>();
            })
            .allow::<A>()
            .clone_entity(e, e_clone1);

        assert_eq!(world.get::<B>(e_clone1), Some(&B(10)));

        let e_clone2 = world.spawn_empty().id();

        EntityCloner::build_opt_in(&mut world)
            .allow::<A>()
            .without_required_components(|builder| {
                builder.allow::<A>();
            })
            .clone_entity(e, e_clone2);

        assert_eq!(world.get::<B>(e_clone2), Some(&B(10)));
    }

    #[test]
    fn clone_required_becoming_explicit() {
        #[derive(Component, Clone, PartialEq, Debug)]
        #[require(B(5))]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct B(u8);

        let mut world = World::default();
        let e = world.spawn((A, B(10))).id();
        let e_clone1 = world.spawn(B(20)).id();
        EntityCloner::build_opt_in(&mut world)
            .allow::<A>()
            .allow::<B>()
            .clone_entity(e, e_clone1);

        assert_eq!(world.get::<B>(e_clone1), Some(&B(10)));

        let e_clone2 = world.spawn(B(20)).id();
        EntityCloner::build_opt_in(&mut world)
            .allow::<A>()
            .allow::<B>()
            .clone_entity(e, e_clone2);

        assert_eq!(world.get::<B>(e_clone2), Some(&B(10)));
    }

    #[test]
    fn required_not_cloned_because_requiring_missing() {
        #[derive(Component, Clone)]
        #[require(B)]
        struct A;

        #[derive(Component, Clone, Default)]
        struct B;

        let mut world = World::default();
        let e = world.spawn(B).id();
        let e_clone1 = world.spawn_empty().id();

        EntityCloner::build_opt_in(&mut world)
            .allow::<A>()
            .clone_entity(e, e_clone1);

        assert!(world.get::<B>(e_clone1).is_none());
    }

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

        EntityCloner::build_opt_in(&mut world)
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

        EntityCloner::build_opt_in(&mut world)
            .without_required_components(|builder| {
                builder.allow::<A>();
            })
            .clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), Some(&A));
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B));
        assert_eq!(world.entity(e_clone).get::<C>(), Some(&C(5)));
    }

    #[test]
    fn clone_entity_with_missing_required_components() {
        #[derive(Component, Clone, PartialEq, Debug)]
        #[require(B)]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(C(5))]
        struct B;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct C(u32);

        let mut world = World::default();

        let e = world.spawn(A).remove::<C>().id();
        let e_clone = world.spawn_empty().id();

        EntityCloner::build_opt_in(&mut world)
            .allow::<A>()
            .clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), Some(&A));
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B));
        assert_eq!(world.entity(e_clone).get::<C>(), Some(&C(5)));
    }

    #[test]
    fn skipped_required_components_counter_is_reset_on_early_return() {
        #[derive(Component, Clone, PartialEq, Debug, Default)]
        #[require(B(5))]
        struct A;

        #[derive(Component, Clone, PartialEq, Debug)]
        struct B(u32);

        #[derive(Component, Clone, PartialEq, Debug, Default)]
        struct C;

        let mut world = World::default();

        let e1 = world.spawn(C).id();
        let e2 = world.spawn((A, B(0))).id();
        let e_clone = world.spawn_empty().id();

        let mut builder = EntityCloner::build_opt_in(&mut world);
        builder.allow::<(A, C)>();
        let mut cloner = builder.finish();
        cloner.clone_entity(&mut world, e1, e_clone);
        cloner.clone_entity(&mut world, e2, e_clone);

        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B(0)));
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
        EntityCloner::build_opt_out(&mut world).clone_entity(entity, entity_clone);

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
        EntityCloner::build_opt_out(&mut world)
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
        assert_eq!(
            (
                world.get::<ChildOf>(root_children[0]),
                world.get::<ChildOf>(root_children[1])
            ),
            (Some(&ChildOf(clone_root)), Some(&ChildOf(clone_root)))
        );
        let child1_children = world.entity(root_children[0]).get::<Children>().unwrap();
        assert_eq!(child1_children.len(), 1);
        assert_ne!(child1_children[0], grandchild);
        assert!(world.entity(root_children[1]).get::<Children>().is_none());
        assert_eq!(
            world.get::<ChildOf>(child1_children[0]),
            Some(&ChildOf(root_children[0]))
        );

        assert_eq!(
            world.entity(root).get::<Children>().unwrap().deref(),
            &[child1, child2]
        );
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

        EntityCloner::build_opt_in(&mut world)
            .allow::<A>()
            .clone_entity(e, e_clone);

        assert_eq!(world.entity(e_clone).get::<A>(), Some(&A));
        assert_eq!(world.entity(e_clone).get::<B>(), Some(&B(1)));
    }

    #[test]
    fn move_without_clone() {
        #[derive(Component, PartialEq, Debug)]
        #[component(storage = "SparseSet")]
        struct A;

        #[derive(Component, PartialEq, Debug)]
        struct B(Vec<u8>);

        let mut world = World::default();
        let e = world.spawn((A, B(alloc::vec![1, 2, 3]))).id();
        let e_clone = world.spawn_empty().id();
        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.move_components(true);
        let mut cloner = builder.finish();

        cloner.clone_entity(&mut world, e, e_clone);

        assert_eq!(world.get::<A>(e), None);
        assert_eq!(world.get::<B>(e), None);

        assert_eq!(world.get::<A>(e_clone), Some(&A));
        assert_eq!(world.get::<B>(e_clone), Some(&B(alloc::vec![1, 2, 3])));
    }

    #[test]
    fn move_with_remove_hook() {
        #[derive(Component, PartialEq, Debug)]
        #[component(on_remove=remove_hook)]
        struct B(Option<Vec<u8>>);

        fn remove_hook(mut world: DeferredWorld, ctx: HookContext) {
            world.get_mut::<B>(ctx.entity).unwrap().0.take();
        }

        let mut world = World::default();
        let e = world.spawn(B(Some(alloc::vec![1, 2, 3]))).id();
        let e_clone = world.spawn_empty().id();
        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.move_components(true);
        let mut cloner = builder.finish();

        cloner.clone_entity(&mut world, e, e_clone);

        assert_eq!(world.get::<B>(e), None);
        assert_eq!(world.get::<B>(e_clone), Some(&B(None)));
    }

    #[test]
    fn move_with_deferred() {
        #[derive(Component, PartialEq, Debug)]
        #[component(clone_behavior=Custom(custom))]
        struct A(u32);

        #[derive(Component, PartialEq, Debug)]
        struct B(u32);

        fn custom(_src: &SourceComponent, ctx: &mut ComponentCloneCtx) {
            // Clone using deferred
            let source = ctx.source();
            ctx.queue_deferred(move |world, mapper| {
                let target = mapper.get_mapped(source);
                world.entity_mut(target).insert(A(10));
            });
        }

        let mut world = World::default();
        let e = world.spawn((A(0), B(1))).id();
        let e_clone = world.spawn_empty().id();
        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.move_components(true);
        let mut cloner = builder.finish();

        cloner.clone_entity(&mut world, e, e_clone);

        assert_eq!(world.get::<A>(e), None);
        assert_eq!(world.get::<A>(e_clone), Some(&A(10)));
        assert_eq!(world.get::<B>(e), None);
        assert_eq!(world.get::<B>(e_clone), Some(&B(1)));
    }

    #[test]
    fn move_relationship() {
        #[derive(Component, Clone, PartialEq, Eq, Debug)]
        #[relationship(relationship_target=Target)]
        struct Source(Entity);

        #[derive(Component, Clone, PartialEq, Eq, Debug)]
        #[relationship_target(relationship=Source)]
        struct Target(Vec<Entity>);

        #[derive(Component, PartialEq, Debug)]
        struct A(u32);

        let mut world = World::default();
        let e_target = world.spawn(A(1)).id();
        let e_source = world.spawn((A(2), Source(e_target))).id();

        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.move_components(true);
        let mut cloner = builder.finish();

        let e_source_moved = world.spawn_empty().id();

        cloner.clone_entity(&mut world, e_source, e_source_moved);

        assert_eq!(world.get::<A>(e_source), None);
        assert_eq!(world.get::<A>(e_source_moved), Some(&A(2)));
        assert_eq!(world.get::<Source>(e_source), None);
        assert_eq!(world.get::<Source>(e_source_moved), Some(&Source(e_target)));
        assert_eq!(
            world.get::<Target>(e_target),
            Some(&Target(alloc::vec![e_source_moved]))
        );

        let e_target_moved = world.spawn_empty().id();

        cloner.clone_entity(&mut world, e_target, e_target_moved);

        assert_eq!(world.get::<A>(e_target), None);
        assert_eq!(world.get::<A>(e_target_moved), Some(&A(1)));
        assert_eq!(world.get::<Target>(e_target), None);
        assert_eq!(
            world.get::<Target>(e_target_moved),
            Some(&Target(alloc::vec![e_source_moved]))
        );
        assert_eq!(
            world.get::<Source>(e_source_moved),
            Some(&Source(e_target_moved))
        );
    }

    #[test]
    fn move_hierarchy() {
        #[derive(Component, PartialEq, Debug)]
        struct A(u32);

        let mut world = World::default();
        let e_parent = world.spawn(A(1)).id();
        let e_child1 = world.spawn((A(2), ChildOf(e_parent))).id();
        let e_child2 = world.spawn((A(3), ChildOf(e_parent))).id();
        let e_child1_1 = world.spawn((A(4), ChildOf(e_child1))).id();

        let e_parent_clone = world.spawn_empty().id();

        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.move_components(true).linked_cloning(true);
        let mut cloner = builder.finish();

        cloner.clone_entity(&mut world, e_parent, e_parent_clone);

        assert_eq!(world.get::<A>(e_parent), None);
        assert_eq!(world.get::<A>(e_child1), None);
        assert_eq!(world.get::<A>(e_child2), None);
        assert_eq!(world.get::<A>(e_child1_1), None);

        let mut children = world.get::<Children>(e_parent_clone).unwrap().iter();
        let e_child1_clone = *children.next().unwrap();
        let e_child2_clone = *children.next().unwrap();
        let mut children = world.get::<Children>(e_child1_clone).unwrap().iter();
        let e_child1_1_clone = *children.next().unwrap();

        assert_eq!(world.get::<A>(e_parent_clone), Some(&A(1)));
        assert_eq!(world.get::<A>(e_child1_clone), Some(&A(2)));
        assert_eq!(
            world.get::<ChildOf>(e_child1_clone),
            Some(&ChildOf(e_parent_clone))
        );
        assert_eq!(world.get::<A>(e_child2_clone), Some(&A(3)));
        assert_eq!(
            world.get::<ChildOf>(e_child2_clone),
            Some(&ChildOf(e_parent_clone))
        );
        assert_eq!(world.get::<A>(e_child1_1_clone), Some(&A(4)));
        assert_eq!(
            world.get::<ChildOf>(e_child1_1_clone),
            Some(&ChildOf(e_child1_clone))
        );
    }

    // Original: E1 Target{target: [E2], data: [4,5,6]}
    //            | E2 Source{target: E1, data: [1,2,3]}
    //
    // Cloned:   E3 Target{target: [], data: [4,5,6]}
    #[test]
    fn clone_relationship_with_data() {
        #[derive(Component, Clone)]
        #[relationship(relationship_target=Target)]
        struct Source {
            #[relationship]
            target: Entity,
            data: Vec<u8>,
        }

        #[derive(Component, Clone)]
        #[relationship_target(relationship=Source)]
        struct Target {
            #[relationship]
            target: Vec<Entity>,
            data: Vec<u8>,
        }

        let mut world = World::default();
        let e_target = world.spawn_empty().id();
        let e_source = world
            .spawn(Source {
                target: e_target,
                data: alloc::vec![1, 2, 3],
            })
            .id();
        world.get_mut::<Target>(e_target).unwrap().data = alloc::vec![4, 5, 6];

        let builder = EntityCloner::build_opt_out(&mut world);
        let mut cloner = builder.finish();

        let e_target_clone = world.spawn_empty().id();
        cloner.clone_entity(&mut world, e_target, e_target_clone);

        let target = world.get::<Target>(e_target).unwrap();
        let cloned_target = world.get::<Target>(e_target_clone).unwrap();

        assert_eq!(cloned_target.data, target.data);
        assert_eq!(target.target, alloc::vec![e_source]);
        assert_eq!(cloned_target.target.len(), 0);

        let source = world.get::<Source>(e_source).unwrap();

        assert_eq!(source.data, alloc::vec![1, 2, 3]);
    }

    // Original: E1 Target{target: [E2], data: [4,5,6]}
    //            | E2 Source{target: E1, data: [1,2,3]}
    //
    // Cloned:   E3 Target{target: [E4], data: [4,5,6]}
    //            | E4 Source{target: E3, data: [1,2,3]}
    #[test]
    fn clone_linked_relationship_with_data() {
        #[derive(Component, Clone)]
        #[relationship(relationship_target=Target)]
        struct Source {
            #[relationship]
            target: Entity,
            data: Vec<u8>,
        }

        #[derive(Component, Clone)]
        #[relationship_target(relationship=Source, linked_spawn)]
        struct Target {
            #[relationship]
            target: Vec<Entity>,
            data: Vec<u8>,
        }

        let mut world = World::default();
        let e_target = world.spawn_empty().id();
        let e_source = world
            .spawn(Source {
                target: e_target,
                data: alloc::vec![1, 2, 3],
            })
            .id();
        world.get_mut::<Target>(e_target).unwrap().data = alloc::vec![4, 5, 6];

        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.linked_cloning(true);
        let mut cloner = builder.finish();

        let e_target_clone = world.spawn_empty().id();
        cloner.clone_entity(&mut world, e_target, e_target_clone);

        let target = world.get::<Target>(e_target).unwrap();
        let cloned_target = world.get::<Target>(e_target_clone).unwrap();

        assert_eq!(cloned_target.data, target.data);
        assert_eq!(target.target, alloc::vec![e_source]);
        assert_eq!(cloned_target.target.len(), 1);

        let source = world.get::<Source>(e_source).unwrap();
        let cloned_source = world.get::<Source>(cloned_target.target[0]).unwrap();

        assert_eq!(cloned_source.data, source.data);
        assert_eq!(source.target, e_target);
        assert_eq!(cloned_source.target, e_target_clone);
    }

    // Original: E1
    //           E2
    //
    // Moved:    E3 Target{target: [], data: [4,5,6]}
    #[test]
    fn move_relationship_with_data() {
        #[derive(Component, Clone, PartialEq, Eq, Debug)]
        #[relationship(relationship_target=Target)]
        struct Source {
            #[relationship]
            target: Entity,
            data: Vec<u8>,
        }

        #[derive(Component, Clone, PartialEq, Eq, Debug)]
        #[relationship_target(relationship=Source)]
        struct Target {
            #[relationship]
            target: Vec<Entity>,
            data: Vec<u8>,
        }

        let source_data = alloc::vec![1, 2, 3];
        let target_data = alloc::vec![4, 5, 6];

        let mut world = World::default();
        let e_target = world.spawn_empty().id();
        let e_source = world
            .spawn(Source {
                target: e_target,
                data: source_data.clone(),
            })
            .id();
        world.get_mut::<Target>(e_target).unwrap().data = target_data.clone();

        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.move_components(true);
        let mut cloner = builder.finish();

        let e_target_moved = world.spawn_empty().id();
        cloner.clone_entity(&mut world, e_target, e_target_moved);

        assert_eq!(world.get::<Target>(e_target), None);
        assert_eq!(
            world.get::<Source>(e_source),
            Some(&Source {
                data: source_data,
                target: e_target_moved,
            })
        );
        assert_eq!(
            world.get::<Target>(e_target_moved),
            Some(&Target {
                target: alloc::vec![e_source],
                data: target_data
            })
        );
    }

    // Original: E1
    //           E2
    //
    // Moved:    E3 Target{target: [E4], data: [4,5,6]}
    //            | E4 Source{target: E3, data: [1,2,3]}
    #[test]
    fn move_linked_relationship_with_data() {
        #[derive(Component, Clone, PartialEq, Eq, Debug)]
        #[relationship(relationship_target=Target)]
        struct Source {
            #[relationship]
            target: Entity,
            data: Vec<u8>,
        }

        #[derive(Component, Clone, PartialEq, Eq, Debug)]
        #[relationship_target(relationship=Source, linked_spawn)]
        struct Target {
            #[relationship]
            target: Vec<Entity>,
            data: Vec<u8>,
        }

        let source_data = alloc::vec![1, 2, 3];
        let target_data = alloc::vec![4, 5, 6];

        let mut world = World::default();
        let e_target = world.spawn_empty().id();
        let e_source = world
            .spawn(Source {
                target: e_target,
                data: source_data.clone(),
            })
            .id();
        world.get_mut::<Target>(e_target).unwrap().data = target_data.clone();

        let mut builder = EntityCloner::build_opt_out(&mut world);
        builder.move_components(true).linked_cloning(true);
        let mut cloner = builder.finish();

        let e_target_moved = world.spawn_empty().id();
        cloner.clone_entity(&mut world, e_target, e_target_moved);

        assert_eq!(world.get::<Target>(e_target), None);
        assert_eq!(world.get::<Source>(e_source), None);

        let moved_target = world.get::<Target>(e_target_moved).unwrap();
        assert_eq!(moved_target.data, target_data);
        assert_eq!(moved_target.target.len(), 1);

        let moved_source = world.get::<Source>(moved_target.target[0]).unwrap();
        assert_eq!(moved_source.data, source_data);
        assert_eq!(moved_source.target, e_target_moved);
    }
}
