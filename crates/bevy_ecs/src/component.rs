//! Types for declaring and storing [`Component`]s.

use crate::{
    self as bevy_ecs,
    change_detection::MAX_CHANGE_AGE,
    storage::{SparseSetIndex, Storages},
    system::{Local, Resource, SystemParam},
    world::{FromWorld, World},
};
pub use bevy_ecs_macros::Component;
use bevy_ptr::{OwningPtr, UnsafeCellDeref};
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use bevy_utils::TypeIdMap;
use std::cell::UnsafeCell;
use std::{
    alloc::Layout,
    any::{Any, TypeId},
    borrow::Cow,
    marker::PhantomData,
    mem::needs_drop,
};

/// A data type that can be used to store data for an [entity].
///
/// `Component` is a [derivable trait]: this means that a data type can implement it by applying a `#[derive(Component)]` attribute to it.
/// However, components must always satisfy the `Send + Sync + 'static` trait bounds.
///
/// [entity]: crate::entity
/// [derivable trait]: https://doc.rust-lang.org/book/appendix-03-derivable-traits.html
///
/// # Examples
///
/// Components can take many forms: they are usually structs, but can also be of every other kind of data type, like enums or zero sized types.
/// The following examples show how components are laid out in code.
///
/// ```
/// # use bevy_ecs::component::Component;
/// # struct Color;
/// #
/// // A component can contain data...
/// #[derive(Component)]
/// struct LicensePlate(String);
///
/// // ... but it can also be a zero-sized marker.
/// #[derive(Component)]
/// struct Car;
///
/// // Components can also be structs with named fields...
/// #[derive(Component)]
/// struct VehiclePerformance {
///     acceleration: f32,
///     top_speed: f32,
///     handling: f32,
/// }
///
/// // ... or enums.
/// #[derive(Component)]
/// enum WheelCount {
///     Two,
///     Three,
///     Four,
/// }
/// ```
///
/// # Component and data access
///
/// See the [`entity`] module level documentation to learn how to add or remove components from an entity.
///
/// See the documentation for [`Query`] to learn how to access component data from a system.
///
/// [`entity`]: crate::entity#usage
/// [`Query`]: crate::system::Query
///
/// # Choosing a storage type
///
/// Components can be stored in the world using different strategies with their own performance implications.
/// By default, components are added to the [`Table`] storage, which is optimized for query iteration.
///
/// Alternatively, components can be added to the [`SparseSet`] storage, which is optimized for component insertion and removal.
/// This is achieved by adding an additional `#[component(storage = "SparseSet")]` attribute to the derive one:
///
/// ```
/// # use bevy_ecs::component::Component;
/// #
/// #[derive(Component)]
/// #[component(storage = "SparseSet")]
/// struct ComponentA;
/// ```
///
/// [`Table`]: crate::storage::Table
/// [`SparseSet`]: crate::storage::SparseSet
///
/// # Implementing the trait for foreign types
///
/// As a consequence of the [orphan rule], it is not possible to separate into two different crates the implementation of `Component` from the definition of a type.
/// This means that it is not possible to directly have a type defined in a third party library as a component.
/// This important limitation can be easily worked around using the [newtype pattern]:
/// this makes it possible to locally define and implement `Component` for a tuple struct that wraps the foreign type.
/// The following example gives a demonstration of this pattern.
///
/// ```
/// // `Component` is defined in the `bevy_ecs` crate.
/// use bevy_ecs::component::Component;
///
/// // `Duration` is defined in the `std` crate.
/// use std::time::Duration;
///
/// // It is not possible to implement `Component` for `Duration` from this position, as they are
/// // both foreign items, defined in an external crate. However, nothing prevents to define a new
/// // `Cooldown` type that wraps `Duration`. As `Cooldown` is defined in a local crate, it is
/// // possible to implement `Component` for it.
/// #[derive(Component)]
/// struct Cooldown(Duration);
/// ```
///
/// [orphan rule]: https://doc.rust-lang.org/book/ch10-02-traits.html#implementing-a-trait-on-a-type
/// [newtype pattern]: https://doc.rust-lang.org/book/ch19-03-advanced-traits.html#using-the-newtype-pattern-to-implement-external-traits-on-external-types
///
/// # `!Sync` Components
/// A `!Sync` type cannot implement `Component`. However, it is possible to wrap a `Send` but not `Sync`
/// type in [`SyncCell`] or the currently unstable [`Exclusive`] to make it `Sync`. This forces only
/// having mutable access (`&mut T` only, never `&T`), but makes it safe to reference across multiple
/// threads.
///
/// This will fail to compile since `RefCell` is `!Sync`.
/// ```compile_fail
/// # use std::cell::RefCell;
/// # use bevy_ecs::component::Component;
/// #[derive(Component)]
/// struct NotSync {
///    counter: RefCell<usize>,
/// }
/// ```
///
/// This will compile since the `RefCell` is wrapped with `SyncCell`.
/// ```
/// # use std::cell::RefCell;
/// # use bevy_ecs::component::Component;
/// use bevy_utils::synccell::SyncCell;
///
/// // This will compile.
/// #[derive(Component)]
/// struct ActuallySync {
///    counter: SyncCell<RefCell<usize>>,
/// }
/// ```
///
/// [`SyncCell`]: bevy_utils::synccell::SyncCell
/// [`Exclusive`]: https://doc.rust-lang.org/nightly/std/sync/struct.Exclusive.html
pub trait Component: Send + Sync + 'static {
    /// A marker type indicating the storage type used for this component.
    /// This must be either [`TableStorage`] or [`SparseStorage`].
    type Storage: ComponentStorage;
}

/// Marker type for components stored in a [`Table`](crate::storage::Table).
pub struct TableStorage;

/// Marker type for components stored in a [`ComponentSparseSet`](crate::storage::ComponentSparseSet).
pub struct SparseStorage;

/// Types used to specify the storage strategy for a component.
///
/// This trait is implemented for [`TableStorage`] and [`SparseStorage`].
/// Custom implementations are forbidden.
pub trait ComponentStorage: sealed::Sealed {
    /// A value indicating the storage strategy specified by this type.
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
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub enum StorageType {
    /// Provides fast and cache-friendly iteration, but slower addition and removal of components.
    /// This is the default storage type.
    #[default]
    Table,
    /// Provides fast addition and removal of components, but slower iteration.
    SparseSet,
}

/// Stores metadata for a type of component or resource stored in a specific [`World`].
#[derive(Debug, Clone)]
pub struct DataInfo {
    id: DataId,
    descriptor: DataDescriptor,
}

impl DataInfo {
    /// Returns a value uniquely identifying the current component.
    #[inline]
    pub fn id(&self) -> DataId {
        self.id
    }

    /// Returns the name of the current component.
    #[inline]
    pub fn name(&self) -> &str {
        &self.descriptor.name
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
    pub(crate) fn new(id: DataId, descriptor: DataDescriptor) -> Self {
        DataInfo { id, descriptor }
    }
}

/// A value which uniquely identifies the type of a [`Component`] within a
/// [`World`].
///
/// Each time a new `Component` type is registered within a `World` using
/// [`World::init_component`](World::init_component) or
/// [`World::init_component_with_descriptor`](World::init_component_with_descriptor),
/// a corresponding `DataId` is created to track it.
///
/// While the distinction between `DataId` and [`TypeId`] may seem superficial, breaking them
/// into two separate but related concepts allows components to exist outside of Rust's type system.
/// Each Rust type registered as a `Component` will have a corresponding `DataId`, but additional
/// `DataId`s may exist in a `World` to track components which cannot be
/// represented as Rust types for scripting or other advanced use-cases.
///
/// A `DataId` is tightly coupled to its parent `World`. Attempting to use a `DataId` from
/// one `World` to access the metadata of a `Component` in a different `World` is undefined behavior
/// and must not be attempted.
///
/// Given a type `T` which implements [`Component`], the `DataId` for `T` can be retrieved
/// from a `World` using [`World::component_id()`] or via [`WorldData::component_id()`]. Access
/// to the `DataId` for a [`Resource`] is available via [`WorldData::resource_id()`].
#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
pub struct DataId(usize);

impl DataId {
    /// Creates a new [`DataId`].
    ///
    /// The `index` is a unique value associated with each type of component in a given world.
    /// Usually, this value is taken from a counter incremented for each type of component registered with the world.
    #[inline]
    pub const fn new(index: usize) -> DataId {
        DataId(index)
    }

    /// Returns the index of the current component.
    #[inline]
    pub fn index(self) -> usize {
        self.0
    }
}

impl SparseSetIndex for DataId {
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
pub struct DataDescriptor {
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
impl std::fmt::Debug for DataDescriptor {
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

impl DataDescriptor {
    // SAFETY: The pointer points to a valid value of type `T` and it is safe to drop this value.
    unsafe fn drop_ptr<T>(x: OwningPtr<'_>) {
        x.drop_as::<T>();
    }

    /// Create a new `ComponentDescriptor` for the type `T`.
    pub fn new<T: Component>() -> Self {
        Self {
            name: Cow::Borrowed(std::any::type_name::<T>()),
            storage_type: T::Storage::STORAGE_TYPE,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
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
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
        }
    }

    fn new_non_send<T: Any>(storage_type: StorageType) -> Self {
        Self {
            name: Cow::Borrowed(std::any::type_name::<T>()),
            storage_type,
            is_send_and_sync: false,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: needs_drop::<T>().then_some(Self::drop_ptr::<T> as _),
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
    pub fn name(&self) -> &str {
        self.name.as_ref()
    }
}

/// Stores metadata associated with each kind of [`Component`] in a given [`World`].
#[derive(Debug, Default)]
pub struct WorldData {
    data_info: Vec<DataInfo>,
    indices: TypeIdMap<DataId>,
    resource_indices: TypeIdMap<DataId>,
}

impl WorldData {
    /// Initializes a component of type `T` with this instance.
    /// If a component of this type has already been initialized, this will return
    /// the ID of the pre-existing component.
    ///
    /// # See also
    ///
    /// * [`WorldData::component_id()`]
    /// * [`WorldData::init_component_with_descriptor()`]
    #[inline]
    pub fn init_component<T: Component>(&mut self, storages: &mut Storages) -> DataId {
        let type_id = TypeId::of::<T>();

        let WorldData {
            indices,
            data_info: components,
            ..
        } = self;
        *indices.entry(type_id).or_insert_with(|| {
            WorldData::init_component_inner(components, storages, DataDescriptor::new::<T>())
        })
    }

    /// Initializes a component described by `descriptor`.
    ///
    /// ## Note
    ///
    /// If this method is called multiple times with identical descriptors, a distinct `DataId`
    /// will be created for each one.
    ///
    /// # See also
    ///
    /// * [`WorldData::component_id()`]
    /// * [`WorldData::init_component()`]
    pub fn init_component_with_descriptor(
        &mut self,
        storages: &mut Storages,
        descriptor: DataDescriptor,
    ) -> DataId {
        WorldData::init_component_inner(&mut self.data_info, storages, descriptor)
    }

    #[inline]
    fn init_component_inner(
        data_info: &mut Vec<DataInfo>,
        storages: &mut Storages,
        descriptor: DataDescriptor,
    ) -> DataId {
        let data_id = DataId(data_info.len());
        let info = DataInfo::new(data_id, descriptor);
        if info.descriptor.storage_type == StorageType::SparseSet {
            storages.sparse_sets.get_or_insert(&info);
        }
        data_info.push(info);
        data_id
    }

    /// Returns the number of components registered with this instance.
    #[inline]
    pub fn len(&self) -> usize {
        self.data_info.len()
    }

    /// Returns `true` if there are no components registered with this instance. Otherwise, this returns `false`.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data_info.len() == 0
    }

    /// Gets the metadata associated with the given component.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_info(&self, id: DataId) -> Option<&DataInfo> {
        self.data_info.get(id.0)
    }

    /// Returns the name associated with the given component.
    ///
    /// This will return an incorrect result if `id` did not come from the same world as `self`. It may return `None` or a garbage value.
    #[inline]
    pub fn get_name(&self, id: DataId) -> Option<&str> {
        self.get_info(id).map(|descriptor| descriptor.name())
    }

    /// Gets the metadata associated with the given component.
    /// # Safety
    ///
    /// `id` must be a valid [`DataId`]
    #[inline]
    pub unsafe fn get_info_unchecked(&self, id: DataId) -> &DataInfo {
        debug_assert!(id.index() < self.data_info.len());
        self.data_info.get_unchecked(id.0)
    }

    /// Type-erased equivalent of [`WorldData::component_id()`].
    #[inline]
    pub fn get_id(&self, type_id: TypeId) -> Option<DataId> {
        self.indices.get(&type_id).copied()
    }

    /// Returns the [`DataId`] of the given [`Component`] type `T`.
    ///
    /// The returned `DataId` is specific to the `WorldData` instance
    /// it was retrieved from and should not be used with another `WorldData`
    /// instance.
    ///
    /// Returns [`None`] if the `Component` type has not
    /// yet been initialized using [`WorldData::init_component()`].
    ///
    /// ```
    /// use bevy_ecs::prelude::*;
    ///
    /// let mut world = World::new();
    ///
    /// #[derive(Component)]
    /// struct ComponentA;
    ///
    /// let component_a_id = world.init_component::<ComponentA>();
    ///
    /// assert_eq!(component_a_id, world.components().component_id::<ComponentA>().unwrap())
    /// ```
    ///
    /// # See also
    ///
    /// * [`WorldData::get_id()`]
    /// * [`WorldData::resource_id()`]
    /// * [`World::component_id()`]
    #[inline]
    pub fn component_id<T: Component>(&self) -> Option<DataId> {
        self.get_id(TypeId::of::<T>())
    }

    /// Type-erased equivalent of [`WorldData::resource_id()`].
    #[inline]
    pub fn get_resource_id(&self, type_id: TypeId) -> Option<DataId> {
        self.resource_indices.get(&type_id).copied()
    }

    /// Returns the [`DataId`] of the given [`Resource`] type `T`.
    ///
    /// The returned `DataId` is specific to the `WorldData` instance
    /// it was retrieved from and should not be used with another `WorldData`
    /// instance.
    ///
    /// Returns [`None`] if the `Resource` type has not
    /// yet been initialized using [`WorldData::init_resource()`].
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
    /// * [`WorldData::component_id()`]
    /// * [`WorldData::get_resource_id()`]
    #[inline]
    pub fn resource_id<T: Resource>(&self) -> Option<DataId> {
        self.get_resource_id(TypeId::of::<T>())
    }

    /// Initializes a [`Resource`] of type `T` with this instance.
    /// If a resource of this type has already been initialized, this will return
    /// the ID of the pre-existing resource.
    ///
    /// # See also
    ///
    /// * [`WorldData::resource_id()`]
    #[inline]
    pub fn init_resource<T: Resource>(&mut self) -> DataId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.get_or_insert_resource_with(TypeId::of::<T>(), || {
                DataDescriptor::new_resource::<T>()
            })
        }
    }

    /// Initializes a [non-send resource](crate::system::NonSend) of type `T` with this instance.
    /// If a resource of this type has already been initialized, this will return
    /// the ID of the pre-existing resource.
    #[inline]
    pub fn init_non_send<T: Any>(&mut self) -> DataId {
        // SAFETY: The [`ComponentDescriptor`] matches the [`TypeId`]
        unsafe {
            self.get_or_insert_resource_with(TypeId::of::<T>(), || {
                DataDescriptor::new_non_send::<T>(StorageType::default())
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
        func: impl FnOnce() -> DataDescriptor,
    ) -> DataId {
        let components = &mut self.data_info;
        *self.resource_indices.entry(type_id).or_insert_with(|| {
            let descriptor = func();
            let component_id = DataId(components.len());
            components.push(DataInfo::new(component_id, descriptor));
            component_id
        })
    }

    /// Gets an iterator over all components registered with this instance.
    pub fn iter(&self) -> impl Iterator<Item = &DataInfo> + '_ {
        self.data_info.iter()
    }
}

/// A value that tracks when a system ran relative to other systems.
/// This is used to power change detection.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
pub struct Tick {
    tick: u32,
}

impl Tick {
    /// The maximum relative age for a change tick.
    /// The value of this is equal to [`MAX_CHANGE_AGE`].
    ///
    /// Since change detection will not work for any ticks older than this,
    /// ticks are periodically scanned to ensure their relative values are below this.
    pub const MAX: Self = Self::new(MAX_CHANGE_AGE);

    /// Creates a new [`Tick`] wrapping the given value.
    #[inline]
    pub const fn new(tick: u32) -> Self {
        Self { tick }
    }

    /// Gets the value of this change tick.
    #[inline]
    pub const fn get(self) -> u32 {
        self.tick
    }

    /// Sets the value of this change tick.
    #[inline]
    pub fn set(&mut self, tick: u32) {
        self.tick = tick;
    }

    /// Returns `true` if this `Tick` occurred since the system's `last_run`.
    ///
    /// `this_run` is the current tick of the system, used as a reference to help deal with wraparound.
    #[inline]
    pub fn is_newer_than(self, last_run: Tick, this_run: Tick) -> bool {
        // This works even with wraparound because the world tick (`this_run`) is always "newer" than
        // `last_run` and `self.tick`, and we scan periodically to clamp `ComponentTicks` values
        // so they never get older than `u32::MAX` (the difference would overflow).
        //
        // The clamp here ensures determinism (since scans could differ between app runs).
        let ticks_since_insert = this_run.relative_to(self).tick.min(MAX_CHANGE_AGE);
        let ticks_since_system = this_run.relative_to(last_run).tick.min(MAX_CHANGE_AGE);

        ticks_since_system > ticks_since_insert
    }

    /// Returns a change tick representing the relationship between `self` and `other`.
    #[inline]
    pub(crate) fn relative_to(self, other: Self) -> Self {
        let tick = self.tick.wrapping_sub(other.tick);
        Self { tick }
    }

    /// Wraps this change tick's value if it exceeds [`Tick::MAX`].
    ///
    /// Returns `true` if wrapping was performed. Otherwise, returns `false`.
    #[inline]
    pub(crate) fn check_tick(&mut self, tick: Tick) -> bool {
        let age = tick.relative_to(*self);
        // This comparison assumes that `age` has not overflowed `u32::MAX` before, which will be true
        // so long as this check always runs before that can happen.
        if age.get() > Self::MAX.get() {
            *self = tick.relative_to(Self::MAX);
            true
        } else {
            false
        }
    }
}

/// Interior-mutable access to the [`Tick`]s for a single component or resource.
#[derive(Copy, Clone, Debug)]
pub struct TickCells<'a> {
    /// The tick indicating when the value was added to the world.
    pub added: &'a UnsafeCell<Tick>,
    /// The tick indicating the last time the value was modified.
    pub changed: &'a UnsafeCell<Tick>,
}

impl<'a> TickCells<'a> {
    /// # Safety
    /// All cells contained within must uphold the safety invariants of [`UnsafeCellDeref::read`].
    #[inline]
    pub(crate) unsafe fn read(&self) -> ComponentTicks {
        ComponentTicks {
            added: self.added.read(),
            changed: self.changed.read(),
        }
    }
}

/// Records when a component was added and when it was last mutably dereferenced (or added).
#[derive(Copy, Clone, Debug)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug))]
pub struct ComponentTicks {
    pub(crate) added: Tick,
    pub(crate) changed: Tick,
}

impl ComponentTicks {
    /// Returns `true` if the component was added after the system last ran.
    #[inline]
    pub fn is_added(&self, last_run: Tick, this_run: Tick) -> bool {
        self.added.is_newer_than(last_run, this_run)
    }

    /// Returns `true` if the component was added or mutably dereferenced after the system last ran.
    #[inline]
    pub fn is_changed(&self, last_run: Tick, this_run: Tick) -> bool {
        self.changed.is_newer_than(last_run, this_run)
    }

    /// Returns the tick recording the time this component was most recently changed.
    #[inline]
    pub fn last_changed_tick(&self) -> Tick {
        self.changed
    }

    /// Returns the tick recording the time this component was added.
    #[inline]
    pub fn added_tick(&self) -> Tick {
        self.added
    }

    pub(crate) fn new(change_tick: Tick) -> Self {
        Self {
            added: change_tick,
            changed: change_tick,
        }
    }

    /// Manually sets the change tick.
    ///
    /// This is normally done automatically via the [`DerefMut`](std::ops::DerefMut) implementation
    /// on [`Mut<T>`](crate::change_detection::Mut), [`ResMut<T>`](crate::change_detection::ResMut), etc.
    /// However, components and resources that make use of interior mutability might require manual updates.
    ///
    /// # Example
    /// ```no_run
    /// # use bevy_ecs::{world::World, component::ComponentTicks};
    /// let world: World = unimplemented!();
    /// let component_ticks: ComponentTicks = unimplemented!();
    ///
    /// component_ticks.set_changed(world.read_change_tick());
    /// ```
    #[inline]
    pub fn set_changed(&mut self, change_tick: Tick) {
        self.changed = change_tick;
    }
}

/// A [`SystemParam`] that provides access to the [`DataId`] for a specific type.
///
/// # Example
/// ```
/// # use bevy_ecs::{system::Local, component::{Component, DataId, ComponentIdFor}};
/// #[derive(Component)]
/// struct Player;
/// fn my_system(component_id: ComponentIdFor<Player>) {
///     let component_id: DataId = component_id.get();
///     // ...
/// }
/// ```
#[derive(SystemParam)]
pub struct ComponentIdFor<'s, T: Component>(Local<'s, InitDataId<T>>);

impl<T: Component> ComponentIdFor<'_, T> {
    /// Gets the [`DataId`] for the type `T`.
    #[inline]
    pub fn get(&self) -> DataId {
        **self
    }
}

impl<T: Component> std::ops::Deref for ComponentIdFor<'_, T> {
    type Target = DataId;
    fn deref(&self) -> &Self::Target {
        &self.0.data_id
    }
}

impl<T: Component> From<ComponentIdFor<'_, T>> for DataId {
    #[inline]
    fn from(to_component_id: ComponentIdFor<T>) -> DataId {
        *to_component_id
    }
}

/// Initializes the [`DataId`] for a specific type when used with [`FromWorld`].
struct InitDataId<T: Component> {
    data_id: DataId,
    marker: PhantomData<T>,
}

impl<T: Component> FromWorld for InitDataId<T> {
    fn from_world(world: &mut World) -> Self {
        Self {
            data_id: world.init_component::<T>(),
            marker: PhantomData,
        }
    }
}
