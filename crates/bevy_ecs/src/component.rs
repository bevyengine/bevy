//! Types for declaring and storing [`Component`]s.

use crate::{
    change_detection::MAX_CHANGE_AGE,
    storage::{SparseSetIndex, Storages},
    system::Resource,
};
pub use bevy_ecs_macros::Component;
use bevy_ptr::OwningPtr;
use std::{
    alloc::Layout,
    any::{Any, TypeId},
    borrow::Cow,
    mem::needs_drop,
};

/// A component is data associated with an [`Entity`](crate::entity::Entity). Each entity can have
/// multiple different types of components, but only one of them per type.
///
/// Any type that is `Send + Sync + 'static` can implement `Component` using `#[derive(Component)]`.
///
/// In order to use foreign types as components, wrap them using a newtype pattern.
/// ```
/// # use bevy_ecs::component::Component;
/// use std::time::Duration;
/// #[derive(Component)]
/// struct Cooldown(Duration);
/// ```
/// Components are added with new entities using [`Commands::spawn`](crate::system::Commands::spawn),
/// or to existing entities with [`EntityCommands::insert`](crate::system::EntityCommands::insert),
/// or their [`World`](crate::world::World) equivalents.
///
/// Components can be accessed in systems by using a [`Query`](crate::system::Query)
/// as one of the arguments.
///
/// Components can be grouped together into a [`Bundle`](crate::bundle::Bundle).
pub trait Component: Send + Sync + 'static {
    type Storage: ComponentStorage;
}

pub struct TableStorage;
pub struct SparseStorage;

pub trait ComponentStorage: sealed::Sealed {
    // because the trait is sealed, those items are private API.
    const STORAGE_TYPE: StorageType;
}

impl ComponentStorage for TableStorage {
    const STORAGE_TYPE: StorageType = StorageType::Table;
}
impl ComponentStorage for SparseStorage {
    const STORAGE_TYPE: StorageType = StorageType::SparseSet;
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::TableStorage {}
    impl Sealed for super::SparseStorage {}
}

/// The storage used for a specific component type.
///
/// # Examples
/// The [`StorageType`] for a component is configured via the derive attribute
///
/// ```
/// # use bevy_ecs::{prelude::*, component::*};
/// #[derive(Component)]
/// #[component(storage = "SparseSet")]
/// struct A;
/// ```
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum StorageType {
    /// Provides fast and cache-friendly iteration, but slower addition and removal of components.
    /// This is the default storage type.
    Table,
    /// Provides fast addition and removal of components, but slower iteration.
    SparseSet,
}

impl Default for StorageType {
    fn default() -> Self {
        StorageType::Table
    }
}

#[derive(Debug)]
pub struct ComponentInfo {
    id: ComponentId,
    descriptor: ComponentDescriptor,
}

impl ComponentInfo {
    #[inline]
    pub fn id(&self) -> ComponentId {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.descriptor.name
    }

    #[inline]
    pub fn type_id(&self) -> Option<TypeId> {
        self.descriptor.type_id
    }

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

    #[inline]
    pub fn storage_type(&self) -> StorageType {
        self.descriptor.storage_type
    }

    #[inline]
    pub fn is_send_and_sync(&self) -> bool {
        self.descriptor.is_send_and_sync
    }

    fn new(id: ComponentId, descriptor: ComponentDescriptor) -> Self {
        ComponentInfo { id, descriptor }
    }
}

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub struct ComponentId(usize);

impl ComponentId {
    #[inline]
    pub const fn new(index: usize) -> ComponentId {
        ComponentId(index)
    }

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

    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

pub struct ComponentDescriptor {
    name: Cow<'static, str>,
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
}

// We need to ignore the `drop` field in our `Debug` impl
impl std::fmt::Debug for ComponentDescriptor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComponentDescriptor")
            .field("name", &self.name)
            .field("storage_type", &self.storage_type)
            .field("is_send_and_sync", &self.is_send_and_sync)
            .field("type_id", &self.type_id)
            .field("layout", &self.layout)
            .finish()
    }
}

impl ComponentDescriptor {
    // SAFETY: The pointer points to a valid value of type `T` and it is safe to drop this value.
    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        x.drop_as::<T>()
    }

    /// Create a new `ComponentDescriptor` for the type `T`.
    pub fn new<T: Component>() -> Self {
        Self {
            name: Cow::Borrowed(std::any::type_name::<T>()),
            storage_type: T::Storage::STORAGE_TYPE,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then(|| Self::drop_ptr::<T> as _),
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
    ) -> Self {
        Self {
            name: name.into(),
            storage_type,
            is_send_and_sync: true,
            type_id: None,
            layout,
            drop,
        }
    }

    /// Create a new `ComponentDescriptor` for a resource.
    ///
    /// The [`StorageType`] for resources is always [`TableStorage`].
    pub fn new_resource<T: Resource>() -> Self {
        Self {
            name: Cow::Borrowed(std::any::type_name::<T>()),
            // PERF: `SparseStorage` may actually be a more
            // reasonable choice as `storage_type` for resources.
            storage_type: StorageType::Table,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then(|| Self::drop_ptr::<T> as _),
        }
    }

    fn new_non_send<T: Any>(storage_type: StorageType) -> Self {
        Self {
            name: Cow::Borrowed(std::any::type_name::<T>()),
            storage_type,
            is_send_and_sync: false,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then(|| Self::drop_ptr::<T> as _),
        }
    }

    #[inline]
    pub fn storage_type(&self) -> StorageType {
        self.storage_type
    }

    #[inline]
    pub fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }

    #[inline]
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

#[derive(Debug, Default)]
pub struct Components {
    components: Vec<ComponentInfo>,
    indices: std::collections::HashMap<TypeId, usize, fxhash::FxBuildHasher>,
    resource_indices: std::collections::HashMap<TypeId, usize, fxhash::FxBuildHasher>,
}

impl Components {
    #[inline]
    pub fn init_component<T: Component>(&mut self, storages: &mut Storages) -> ComponentId {
        let type_id = TypeId::of::<T>();

        let Components {
            indices,
            components,
            ..
        } = self;
        let index = indices.entry(type_id).or_insert_with(|| {
            Components::init_component_inner(components, storages, ComponentDescriptor::new::<T>())
        });
        ComponentId(*index)
    }

    pub fn init_component_with_descriptor(
        &mut self,
        storages: &mut Storages,
        descriptor: ComponentDescriptor,
    ) -> ComponentId {
        let index = Components::init_component_inner(&mut self.components, storages, descriptor);
        ComponentId(index)
    }

    #[inline]
    fn init_component_inner(
        components: &mut Vec<ComponentInfo>,
        storages: &mut Storages,
        descriptor: ComponentDescriptor,
    ) -> usize {
        let index = components.len();
        let info = ComponentInfo::new(ComponentId(index), descriptor);
        if info.descriptor.storage_type == StorageType::SparseSet {
            storages.sparse_sets.get_or_insert(&info);
        }
        components.push(info);
        index
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.components.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.components.len() == 0
    }

    #[inline]
    pub fn get_info(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.components.get(id.0)
    }

    /// # Safety
    ///
    /// `id` must be a valid [`ComponentId`]
    #[inline]
    pub unsafe fn get_info_unchecked(&self, id: ComponentId) -> &ComponentInfo {
        debug_assert!(id.index() < self.components.len());
        self.components.get_unchecked(id.0)
    }

    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.indices.get(&type_id).map(|index| ComponentId(*index))
    }

    #[inline]
    pub fn get_resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.resource_indices
            .get(&type_id)
            .map(|index| ComponentId(*index))
    }

    #[inline]
    pub fn init_resource<T: Resource>(&mut self) -> ComponentId {
        // SAFE: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.get_or_insert_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_resource::<T>()
            })
        }
    }

    #[inline]
    pub fn init_non_send<T: Any>(&mut self) -> ComponentId {
        // SAFE: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.get_or_insert_resource_with(TypeId::of::<T>(), || {
                ComponentDescriptor::new_non_send::<T>(StorageType::default())
            })
        }
    }

    /// # Safety
    ///
    /// The [`ComponentDescriptor`] must match the [`TypeId`]
    #[inline]
    unsafe fn get_or_insert_resource_with(
        &mut self,
        type_id: TypeId,
        func: impl FnOnce() -> ComponentDescriptor,
    ) -> ComponentId {
        let components = &mut self.components;
        let index = self.resource_indices.entry(type_id).or_insert_with(|| {
            let descriptor = func();
            let index = components.len();
            components.push(ComponentInfo::new(ComponentId(index), descriptor));
            index
        });

        ComponentId(*index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &ComponentInfo> + '_ {
        self.components.iter()
    }
}

/// Records when a component was added and when it was last mutably dereferenced (or added).
#[derive(Copy, Clone, Debug)]
pub struct ComponentTicks {
    pub(crate) added: u32,
    pub(crate) changed: u32,
}

impl ComponentTicks {
    #[inline]
    /// Returns `true` if the component was added after the system last ran.
    pub fn is_added(&self, last_change_tick: u32, change_tick: u32) -> bool {
        // This works even with wraparound because the world tick (`change_tick`) is always "newer" than
        // `last_change_tick` and `self.added`, and we scan periodically to clamp `ComponentTicks` values
        // so they never get older than `u32::MAX` (the difference would overflow).
        //
        // The clamp here ensures determinism (since scans could differ between app runs).
        let ticks_since_insert = change_tick.wrapping_sub(self.added).min(MAX_CHANGE_AGE);
        let ticks_since_system = change_tick
            .wrapping_sub(last_change_tick)
            .min(MAX_CHANGE_AGE);

        ticks_since_system > ticks_since_insert
    }

    #[inline]
    /// Returns `true` if the component was added or mutably dereferenced after the system last ran.
    pub fn is_changed(&self, last_change_tick: u32, change_tick: u32) -> bool {
        // This works even with wraparound because the world tick (`change_tick`) is always "newer" than
        // `last_change_tick` and `self.changed`, and we scan periodically to clamp `ComponentTicks` values
        // so they never get older than `u32::MAX` (the difference would overflow).
        //
        // The clamp here ensures determinism (since scans could differ between app runs).
        let ticks_since_change = change_tick.wrapping_sub(self.changed).min(MAX_CHANGE_AGE);
        let ticks_since_system = change_tick
            .wrapping_sub(last_change_tick)
            .min(MAX_CHANGE_AGE);

        ticks_since_system > ticks_since_change
    }

    pub(crate) fn new(change_tick: u32) -> Self {
        Self {
            added: change_tick,
            changed: change_tick,
        }
    }

    pub(crate) fn check_ticks(&mut self, change_tick: u32) {
        check_tick(&mut self.added, change_tick);
        check_tick(&mut self.changed, change_tick);
    }

    /// Manually sets the change tick.
    ///
    /// This is normally done automatically via the [`DerefMut`](std::ops::DerefMut) implementation
    /// on [`Mut<T>`](crate::change_detection::Mut), [`ResMut<T>`](crate::change_detection::ResMut), etc.
    /// However, components and resources that make use of interior mutability might require manual updates.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use bevy_ecs::{world::World, component::ComponentTicks};
    /// let world: World = unimplemented!();
    /// let component_ticks: ComponentTicks = unimplemented!();
    ///
    /// component_ticks.set_changed(world.read_change_tick());
    /// ```
    #[inline]
    pub fn set_changed(&mut self, change_tick: u32) {
        self.changed = change_tick;
    }
}

fn check_tick(last_change_tick: &mut u32, change_tick: u32) {
    let age = change_tick.wrapping_sub(*last_change_tick);
    // This comparison assumes that `age` has not overflowed `u32::MAX` before, which will be true
    // so long as this check always runs before that can happen.
    if age > MAX_CHANGE_AGE {
        *last_change_tick = change_tick.wrapping_sub(MAX_CHANGE_AGE);
    }
}
