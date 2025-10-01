use alloc::{borrow::Cow, vec::Vec};
use bevy_platform::{hash::FixedHasher, sync::PoisonError};
use bevy_ptr::OwningPtr;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::{prelude::DebugName, TypeIdMap};
use core::{
    alloc::Layout,
    any::{Any, TypeId},
    fmt::Debug,
    mem::needs_drop,
};
use indexmap::IndexSet;

use crate::{
    archetype::ArchetypeFlags,
    component::{
        Component, ComponentCloneBehavior, ComponentMutability, QueuedComponents,
        RequiredComponents, StorageType,
    },
    lifecycle::ComponentHooks,
    query::DebugCheckedUnwrap as _,
    resource::Resource,
    storage::SparseSetIndex,
};

/// Stores metadata for a type of component or resource stored in a specific [`World`](crate::world::World).
#[derive(Debug, Clone)]
pub struct ComponentInfo {
    pub(super) id: ComponentId,
    pub(super) descriptor: ComponentDescriptor,
    pub(super) hooks: ComponentHooks,
    pub(super) required_components: RequiredComponents,
    /// The set of components that require this components.
    /// Invariant: components in this set always appear after the components that they require.
    pub(super) required_by: IndexSet<ComponentId, FixedHasher>,
}

impl ComponentInfo {
    /// Returns a value uniquely identifying the current component.
    #[inline]
    pub fn id(&self) -> ComponentId {
        self.id
    }

    /// Returns the name of the current component.
    #[inline]
    pub fn name(&self) -> DebugName {
        self.descriptor.name.clone()
    }

    /// Returns `true` if the current component is mutable.
    #[inline]
    pub fn mutable(&self) -> bool {
        self.descriptor.mutable
    }

    /// Returns [`ComponentCloneBehavior`] of the current component.
    #[inline]
    pub fn clone_behavior(&self) -> &ComponentCloneBehavior {
        &self.descriptor.clone_behavior
    }

    /// Returns the [`TypeId`] of the underlying component type.
    /// Returns `None` if the component does not correspond to a Rust type.
    #[inline]
    pub fn type_id(&self) -> Option<TypeId> {
        self.descriptor.type_id
    }

    /// Returns the layout used to store values of this component in memory.
    #[inline]
    pub fn layout(&self) -> Layout {
        self.descriptor.layout
    }

    #[inline]
    /// Get the function which should be called to clean up values of
    /// the underlying component type. This maps to the
    /// [`Drop`] implementation for 'normal' Rust components
    ///
    /// Returns `None` if values of the underlying component type don't
    /// need to be dropped, e.g. as reported by [`needs_drop`].
    pub fn drop(&self) -> Option<unsafe fn(OwningPtr<'_>)> {
        self.descriptor.drop
    }

    /// Returns a value indicating the storage strategy for the current component.
    #[inline]
    pub fn storage_type(&self) -> StorageType {
        self.descriptor.storage_type
    }

    /// Returns `true` if the underlying component type can be freely shared between threads.
    /// If this returns `false`, then extra care must be taken to ensure that components
    /// are not accessed from the wrong thread.
    #[inline]
    pub fn is_send_and_sync(&self) -> bool {
        self.descriptor.is_send_and_sync
    }

    /// Create a new [`ComponentInfo`].
    pub(crate) fn new(id: ComponentId, descriptor: ComponentDescriptor) -> Self {
        ComponentInfo {
            id,
            descriptor,
            hooks: Default::default(),
            required_components: Default::default(),
            required_by: Default::default(),
        }
    }

    /// Update the given flags to include any [`ComponentHook`](crate::component::ComponentHook) registered to self
    #[inline]
    pub(crate) fn update_archetype_flags(&self, flags: &mut ArchetypeFlags) {
        if self.hooks().on_add.is_some() {
            flags.insert(ArchetypeFlags::ON_ADD_HOOK);
        }
        if self.hooks().on_insert.is_some() {
            flags.insert(ArchetypeFlags::ON_INSERT_HOOK);
        }
        if self.hooks().on_replace.is_some() {
            flags.insert(ArchetypeFlags::ON_REPLACE_HOOK);
        }
        if self.hooks().on_remove.is_some() {
            flags.insert(ArchetypeFlags::ON_REMOVE_HOOK);
        }
        if self.hooks().on_despawn.is_some() {
            flags.insert(ArchetypeFlags::ON_DESPAWN_HOOK);
        }
    }

    /// Provides a reference to the collection of hooks associated with this [`Component`]
    pub fn hooks(&self) -> &ComponentHooks {
        &self.hooks
    }

    /// Retrieves the [`RequiredComponents`] collection, which contains all required components (and their constructors)
    /// needed by this component. This includes _recursive_ required components.
    pub fn required_components(&self) -> &RequiredComponents {
        &self.required_components
    }
}

/// A value which uniquely identifies the type of a [`Component`] or [`Resource`] within a
/// [`World`](crate::world::World).
///
/// Each time a new `Component` type is registered within a `World` using
/// e.g. [`World::register_component`](crate::world::World::register_component) or
/// [`World::register_component_with_descriptor`](crate::world::World::register_component_with_descriptor)
/// or a Resource with e.g. [`World::init_resource`](crate::world::World::init_resource),
/// a corresponding `ComponentId` is created to track it.
///
/// While the distinction between `ComponentId` and [`TypeId`] may seem superficial, breaking them
/// into two separate but related concepts allows components to exist outside of Rust's type system.
/// Each Rust type registered as a `Component` will have a corresponding `ComponentId`, but additional
/// `ComponentId`s may exist in a `World` to track components which cannot be
/// represented as Rust types for scripting or other advanced use-cases.
///
/// A `ComponentId` is tightly coupled to its parent `World`. Attempting to use a `ComponentId` from
/// one `World` to access the metadata of a `Component` in a different `World` is undefined behavior
/// and must not be attempted.
///
/// Given a type `T` which implements [`Component`], the `ComponentId` for `T` can be retrieved
/// from a `World` using [`World::component_id()`](crate::world::World::component_id) or via [`Components::component_id()`].
/// Access to the `ComponentId` for a [`Resource`] is available via [`Components::resource_id()`].
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq, Clone)
)]
pub struct ComponentId(pub(super) usize);

impl ComponentId {
    /// Creates a new [`ComponentId`].
    ///
    /// The `index` is a unique value associated with each type of component in a given world.
    /// Usually, this value is taken from a counter incremented for each type of component registered with the world.
    #[inline]
    pub const fn new(index: usize) -> ComponentId {
        ComponentId(index)
    }

    /// Returns the index of the current component.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

impl SparseSetIndex for ComponentId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.index()
    }

    #[inline]
    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

/// A value describing a component or resource, which may or may not correspond to a Rust type.
#[derive(Clone)]
pub struct ComponentDescriptor {
    name: DebugName,
    // SAFETY: This must remain private. It must match the statically known StorageType of the
    // associated rust component type if one exists.
    storage_type: StorageType,
    // SAFETY: This must remain private. It must only be set to "true" if this component is
    // actually Send + Sync
    is_send_and_sync: bool,
    type_id: Option<TypeId>,
    layout: Layout,
    // SAFETY: this function must be safe to call with pointers pointing to items of the type
    // this descriptor describes.
    // None if the underlying type doesn't need to be dropped
    drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>,
    mutable: bool,
    clone_behavior: ComponentCloneBehavior,
}

// We need to ignore the `drop` field in our `Debug` impl
impl Debug for ComponentDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ComponentDescriptor")
            .field("name", &self.name)
            .field("storage_type", &self.storage_type)
            .field("is_send_and_sync", &self.is_send_and_sync)
            .field("type_id", &self.type_id)
            .field("layout", &self.layout)
            .field("mutable", &self.mutable)
            .field("clone_behavior", &self.clone_behavior)
            .finish()
    }
}

impl ComponentDescriptor {
    /// # Safety
    ///
    /// `x` must point to a valid value of type `T`.
    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        // SAFETY: Contract is required to be upheld by the caller.
        unsafe {
            x.drop_as::<T>();
        }
    }

    /// Create a new `ComponentDescriptor` for the type `T`.
    pub fn new<T: Component>() -> Self {
        Self {
            name: DebugName::type_name::<T>(),
            storage_type: T::STORAGE_TYPE,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
            mutable: T::Mutability::MUTABLE,
            clone_behavior: T::clone_behavior(),
        }
    }

    /// Create a new `ComponentDescriptor`.
    ///
    /// # Safety
    /// - the `drop` fn must be usable on a pointer with a value of the layout `layout`
    /// - the component type must be safe to access from any thread (Send + Sync in rust terms)
    pub unsafe fn new_with_layout(
        name: impl Into<Cow<'static, str>>,
        storage_type: StorageType,
        layout: Layout,
        drop: Option<for<'a> unsafe fn(OwningPtr<'a>)>,
        mutable: bool,
        clone_behavior: ComponentCloneBehavior,
    ) -> Self {
        Self {
            name: name.into().into(),
            storage_type,
            is_send_and_sync: true,
            type_id: None,
            layout,
            drop,
            mutable,
            clone_behavior,
        }
    }

    /// Create a new `ComponentDescriptor` for a resource.
    ///
    /// The [`StorageType`] for resources is always [`StorageType::Table`].
    pub fn new_resource<T: Resource>() -> Self {
        Self {
            name: DebugName::type_name::<T>(),
            // PERF: `SparseStorage` may actually be a more
            // reasonable choice as `storage_type` for resources.
            storage_type: StorageType::Table,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
            mutable: true,
            clone_behavior: ComponentCloneBehavior::Default,
        }
    }

    pub(super) fn new_non_send<T: Any>(storage_type: StorageType) -> Self {
        Self {
            name: DebugName::type_name::<T>(),
            storage_type,
            is_send_and_sync: false,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
            mutable: true,
            clone_behavior: ComponentCloneBehavior::Default,
        }
    }

    /// Returns a value indicating the storage strategy for the current component.
    #[inline]
    pub fn storage_type(&self) -> StorageType {
        self.storage_type
    }

    /// Returns the [`TypeId`] of the underlying component type.
    /// Returns `None` if the component does not correspond to a Rust type.
    #[inline]
    pub fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }

    /// Returns the name of the current component.
    #[inline]
    pub fn name(&self) -> DebugName {
        self.name.clone()
    }

    /// Returns whether this component is mutable.
    #[inline]
    pub fn mutable(&self) -> bool {
        self.mutable
    }
}

/// Stores metadata associated with each kind of [`Component`] in a given [`World`](crate::world::World).
#[derive(Debug, Default)]
pub struct Components {
    pub(super) components: Vec<Option<ComponentInfo>>,
    pub(super) indices: TypeIdMap<ComponentId>,
    pub(super) resource_indices: TypeIdMap<ComponentId>,
    // This is kept internal and local to verify that no deadlocks can occor.
    pub(super) queued: bevy_platform::sync::RwLock<QueuedComponents>,
}

impl Components {
    /// This registers any descriptor, component or resource.
    ///
    /// # Safety
    ///
    /// The id must have never been registered before. This must be a fresh registration.
    #[inline]
    pub(super) unsafe fn register_component_inner(
        &mut self,
        id: ComponentId,
        descriptor: ComponentDescriptor,
    ) {
        let info = ComponentInfo::new(id, descriptor);
        let least_len = id.0 + 1;
        if self.components.len() < least_len {
            self.components.resize_with(least_len, || None);
        }
        // SAFETY: We just extended the vec to make this index valid.
        let slot = unsafe { self.components.get_mut(id.0).debug_checked_unwrap() };
        // Caller ensures id is unique
        debug_assert!(slot.is_none());
        *slot = Some(info);
    }

    /// Returns the number of components registered or queued with this instance.
    #[inline]
    pub fn len(&self) -> usize {
        self.num_queued() + self.num_registered()
    }

    /// Returns `true` if there are no components registered or queued with this instance. Otherwise, this returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of components registered with this instance.
    #[inline]
    pub fn num_queued(&self) -> usize {
        let queued = self.queued.read().unwrap_or_else(PoisonError::into_inner);
        queued.components.len() + queued.dynamic_registrations.len() + queued.resources.len()
    }

    /// Returns `true` if there are any components registered with this instance. Otherwise, this returns `false`.
    #[inline]
    pub fn any_queued(&self) -> bool {
        self.num_queued() > 0
    }

    /// A faster version of [`Self::num_queued`].
    #[inline]
    pub fn num_queued_mut(&mut self) -> usize {
        let queued = self
            .queued
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        queued.components.len() + queued.dynamic_registrations.len() + queued.resources.len()
    }

    /// A faster version of [`Self::any_queued`].
    #[inline]
    pub fn any_queued_mut(&mut self) -> bool {
        self.num_queued_mut() > 0
    }

    /// Returns the number of components registered with this instance.
    #[inline]
    pub fn num_registered(&self) -> usize {
        self.components.len()
    }

    /// Returns `true` if there are any components registered with this instance. Otherwise, this returns `false`.
    #[inline]
    pub fn any_registered(&self) -> bool {
        self.num_registered() > 0
    }

    /// Gets the metadata associated with the given component, if it is registered.
    /// This will return `None` if the id is not registered or is queued.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_info(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.components.get(id.0).and_then(|info| info.as_ref())
    }

    /// Gets the [`ComponentDescriptor`] of the component with this [`ComponentId`] if it is present.
    /// This will return `None` only if the id is neither registered nor queued to be registered.
    ///
    /// Currently, the [`Cow`] will be [`Cow::Owned`] if and only if the component is queued. It will be [`Cow::Borrowed`] otherwise.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_descriptor<'a>(&'a self, id: ComponentId) -> Option<Cow<'a, ComponentDescriptor>> {
        self.components
            .get(id.0)
            .and_then(|info| info.as_ref().map(|info| Cow::Borrowed(&info.descriptor)))
            .or_else(|| {
                let queued = self.queued.read().unwrap_or_else(PoisonError::into_inner);
                // first check components, then resources, then dynamic
                queued
                    .components
                    .values()
                    .chain(queued.resources.values())
                    .chain(queued.dynamic_registrations.iter())
                    .find(|queued| queued.id == id)
                    .map(|queued| Cow::Owned(queued.descriptor.clone()))
            })
    }

    /// Gets the name of the component with this [`ComponentId`] if it is present.
    /// This will return `None` only if the id is neither registered nor queued to be registered.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_name<'a>(&'a self, id: ComponentId) -> Option<DebugName> {
        self.components
            .get(id.0)
            .and_then(|info| info.as_ref().map(|info| info.descriptor.name()))
            .or_else(|| {
                let queued = self.queued.read().unwrap_or_else(PoisonError::into_inner);
                // first check components, then resources, then dynamic
                queued
                    .components
                    .values()
                    .chain(queued.resources.values())
                    .chain(queued.dynamic_registrations.iter())
                    .find(|queued| queued.id == id)
                    .map(|queued| queued.descriptor.name.clone())
            })
    }

    /// Gets the metadata associated with the given component.
    /// # Safety
    ///
    /// `id` must be a valid and fully registered [`ComponentId`].
    #[inline]
    pub unsafe fn get_info_unchecked(&self, id: ComponentId) -> &ComponentInfo {
        // SAFETY: The caller ensures `id` is valid.
        unsafe {
            self.components
                .get(id.0)
                .debug_checked_unwrap()
                .as_ref()
                .debug_checked_unwrap()
        }
    }

    #[inline]
    pub(crate) fn get_hooks_mut(&mut self, id: ComponentId) -> Option<&mut ComponentHooks> {
        self.components
            .get_mut(id.0)
            .and_then(|info| info.as_mut().map(|info| &mut info.hooks))
    }

    #[inline]
    pub(crate) fn get_required_components(&self, id: ComponentId) -> Option<&RequiredComponents> {
        self.components
            .get(id.0)
            .and_then(|info| info.as_ref().map(|info| &info.required_components))
    }

    #[inline]
    pub(crate) fn get_required_components_mut(
        &mut self,
        id: ComponentId,
    ) -> Option<&mut RequiredComponents> {
        self.components
            .get_mut(id.0)
            .and_then(|info| info.as_mut().map(|info| &mut info.required_components))
    }

    #[inline]
    pub(crate) fn get_required_by(
        &self,
        id: ComponentId,
    ) -> Option<&IndexSet<ComponentId, FixedHasher>> {
        self.components
            .get(id.0)
            .and_then(|info| info.as_ref().map(|info| &info.required_by))
    }

    #[inline]
    pub(crate) fn get_required_by_mut(
        &mut self,
        id: ComponentId,
    ) -> Option<&mut IndexSet<ComponentId, FixedHasher>> {
        self.components
            .get_mut(id.0)
            .and_then(|info| info.as_mut().map(|info| &mut info.required_by))
    }

    /// Returns true if the [`ComponentId`] is fully registered and valid.
    /// Ids may be invalid if they are still queued to be registered.
    /// Those ids are still correct, but they are not usable in every context yet.
    #[inline]
    pub fn is_id_valid(&self, id: ComponentId) -> bool {
        self.components.get(id.0).is_some_and(Option::is_some)
    }

    /// Type-erased equivalent of [`Components::valid_component_id()`].
    #[inline]
    pub fn get_valid_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.indices.get(&type_id).copied()
    }

    /// Returns the [`ComponentId`] of the given [`Component`] type `T` if it is fully registered.
    /// If you want to include queued registration, see [`Components::component_id()`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Component)]
    /// struct ComponentA;
    ///
    /// let component_a_id = world.register_component::<ComponentA>();
    ///
    /// assert_eq!(component_a_id, world.components().valid_component_id::<ComponentA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::get_valid_id()`]
    /// * [`Components::valid_resource_id()`]
    /// * [`World::component_id()`](crate::world::World::component_id)
    #[inline]
    pub fn valid_component_id<T: Component>(&self) -> Option<ComponentId> {
        self.get_valid_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`Components::valid_resource_id()`].
    #[inline]
    pub fn get_valid_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.resource_indices.get(&type_id).copied()
    }

    /// Returns the [`ComponentId`] of the given [`Resource`] type `T` if it is fully registered.
    /// If you want to include queued registration, see [`Components::resource_id()`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Resource, Default)]
    /// struct ResourceA;
    ///
    /// let resource_a_id = world.init_resource::<ResourceA>();
    ///
    /// assert_eq!(resource_a_id, world.components().valid_resource_id::<ResourceA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::valid_component_id()`]
    /// * [`Components::get_resource_id()`]
    #[inline]
    pub fn valid_resource_id<T: Resource>(&self) -> Option<ComponentId> {
        self.get_valid_resource_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`Components::component_id()`].
    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.indices.get(&type_id).copied().or_else(|| {
            self.queued
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .components
                .get(&type_id)
                .map(|queued| queued.id)
        })
    }

    /// Returns the [`ComponentId`] of the given [`Component`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `Components` instance
    /// it was retrieved from and should not be used with another `Components`
    /// instance.
    ///
    /// Returns [`None`] if the `Component` type has not yet been initialized using
    /// [`ComponentsRegistrator::register_component()`](super::ComponentsRegistrator::register_component) or
    /// [`ComponentsQueuedRegistrator::queue_register_component()`](super::ComponentsQueuedRegistrator::queue_register_component).
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Component)]
    /// struct ComponentA;
    ///
    /// let component_a_id = world.register_component::<ComponentA>();
    ///
    /// assert_eq!(component_a_id, world.components().component_id::<ComponentA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`ComponentIdFor`](super::ComponentIdFor)
    /// * [`Components::get_id()`]
    /// * [`Components::resource_id()`]
    /// * [`World::component_id()`](crate::world::World::component_id)
    #[inline]
    pub fn component_id<T: Component>(&self) -> Option<ComponentId> {
        self.get_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`Components::resource_id()`].
    #[inline]
    pub fn get_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.resource_indices.get(&type_id).copied().or_else(|| {
            self.queued
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .resources
                .get(&type_id)
                .map(|queued| queued.id)
        })
    }

    /// Returns the [`ComponentId`] of the given [`Resource`] type `T`.
    ///
    /// The returned `ComponentId` is specific to the `Components` instance
    /// it was retrieved from and should not be used with another `Components`
    /// instance.
    ///
    /// Returns [`None`] if the `Resource` type has not yet been initialized using
    /// [`ComponentsRegistrator::register_resource()`](super::ComponentsRegistrator::register_resource) or
    /// [`ComponentsQueuedRegistrator::queue_register_resource()`](super::ComponentsQueuedRegistrator::queue_register_resource).
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Resource, Default)]
    /// struct ResourceA;
    ///
    /// let resource_a_id = world.init_resource::<ResourceA>();
    ///
    /// assert_eq!(resource_a_id, world.components().resource_id::<ResourceA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`Components::component_id()`]
    /// * [`Components::get_resource_id()`]
    #[inline]
    pub fn resource_id<T: Resource>(&self) -> Option<ComponentId> {
        self.get_resource_id(TypeId::of::<T>())
    }

    /// # Safety
    ///
    /// The [`ComponentDescriptor`] must match the [`TypeId`].
    /// The [`ComponentId`] must be unique.
    /// The [`TypeId`] and [`ComponentId`] must not be registered or queued.
    #[inline]
    pub(super) unsafe fn register_resource_unchecked(
        &mut self,
        type_id: TypeId,
        component_id: ComponentId,
        descriptor: ComponentDescriptor,
    ) {
        // SAFETY: ensured by caller
        unsafe {
            self.register_component_inner(component_id, descriptor);
        }
        let prev = self.resource_indices.insert(type_id, component_id);
        debug_assert!(prev.is_none());
    }

    /// Gets an iterator over all components fully registered with this instance.
    pub fn iter_registered(&self) -> impl Iterator<Item = &ComponentInfo> + '_ {
        self.components.iter().filter_map(Option::as_ref)
    }
}
