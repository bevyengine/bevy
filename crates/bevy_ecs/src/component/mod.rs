mod type_info;

pub use type_info::*;

use crate::storage::SparseSetIndex;
use std::{
    alloc::Layout,
    any::{Any, TypeId},
    collections::hash_map::Entry,
};
use thiserror::Error;

/// A component is data associated with an [`Entity`](crate::entity::Entity). Each entity can have
/// multiple different types of components, but only one of them per type.
///
/// Any type that is `Send + Sync + 'static` automatically implements `Component`.
///
/// Components are added with new entities using [`Commands::spawn`](crate::system::Commands::spawn),
/// or to existing entities with [`EntityCommands::insert`](crate::system::EntityCommands::insert),
/// or their [`World`](crate::world::World) equivalents.
///
/// Components can be accessed in systems by using a [`Query`](crate::system::Query)
/// as one of the arguments.
///
/// Components can be grouped together into a [`Bundle`](crate::bundle::Bundle).
pub trait Component: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Component for T {}

/// The storage used for a specific component type.
///
/// # Examples
/// The [`StorageType`] for a component is normally configured via `World::register_component`.
///
/// ```
/// # use bevy_ecs::{prelude::*, component::*};
///
/// struct A;
///
/// let mut world = World::default();
/// world.register_component(ComponentDescriptor::new::<A>(StorageType::SparseSet));
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
    name: String,
    id: ComponentId,
    type_id: Option<TypeId>,
    // SAFETY: This must remain private. It must only be set to "true" if this component is
    // actually Send + Sync
    is_send_and_sync: bool,
    layout: Layout,
    drop: unsafe fn(*mut u8),
    storage_type: StorageType,
}

impl ComponentInfo {
    #[inline]
    pub fn id(&self) -> ComponentId {
        self.id
    }

    #[inline]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[inline]
    pub fn type_id(&self) -> Option<TypeId> {
        self.type_id
    }

    #[inline]
    pub fn layout(&self) -> Layout {
        self.layout
    }

    #[inline]
    pub fn drop(&self) -> unsafe fn(*mut u8) {
        self.drop
    }

    #[inline]
    pub fn storage_type(&self) -> StorageType {
        self.storage_type
    }

    #[inline]
    pub fn is_send_and_sync(&self) -> bool {
        self.is_send_and_sync
    }

    fn new(id: ComponentId, descriptor: ComponentDescriptor) -> Self {
        ComponentInfo {
            id,
            name: descriptor.name,
            storage_type: descriptor.storage_type,
            type_id: descriptor.type_id,
            is_send_and_sync: descriptor.is_send_and_sync,
            drop: descriptor.drop,
            layout: descriptor.layout,
        }
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
    name: String,
    storage_type: StorageType,
    // SAFETY: This must remain private. It must only be set to "true" if this component is
    // actually Send + Sync
    is_send_and_sync: bool,
    type_id: Option<TypeId>,
    layout: Layout,
    drop: unsafe fn(*mut u8),
}

impl ComponentDescriptor {
    pub fn new<T: Component>(storage_type: StorageType) -> Self {
        Self {
            name: std::any::type_name::<T>().to_string(),
            storage_type,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: TypeInfo::drop_ptr::<T>,
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
        &self.name
    }
}

impl From<TypeInfo> for ComponentDescriptor {
    fn from(type_info: TypeInfo) -> Self {
        Self {
            name: type_info.type_name().to_string(),
            storage_type: StorageType::default(),
            is_send_and_sync: type_info.is_send_and_sync(),
            type_id: Some(type_info.type_id()),
            drop: type_info.drop(),
            layout: type_info.layout(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Components {
    components: Vec<ComponentInfo>,
    indices: std::collections::HashMap<TypeId, usize, fxhash::FxBuildHasher>,
    resource_indices: std::collections::HashMap<TypeId, usize, fxhash::FxBuildHasher>,
}

#[derive(Debug, Error)]
pub enum ComponentsError {
    #[error("A component of type {name:?} ({type_id:?}) already exists")]
    ComponentAlreadyExists { type_id: TypeId, name: String },
}

impl Components {
    pub(crate) fn add(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> Result<ComponentId, ComponentsError> {
        let index = self.components.len();
        if let Some(type_id) = descriptor.type_id {
            let index_entry = self.indices.entry(type_id);
            if let Entry::Occupied(_) = index_entry {
                return Err(ComponentsError::ComponentAlreadyExists {
                    type_id,
                    name: descriptor.name,
                });
            }
            self.indices.insert(type_id, index);
        }
        self.components
            .push(ComponentInfo::new(ComponentId(index), descriptor));

        Ok(ComponentId(index))
    }

    #[inline]
    pub fn get_or_insert_id<T: Component>(&mut self) -> ComponentId {
        self.get_or_insert_with(TypeId::of::<T>(), TypeInfo::of::<T>)
    }

    #[inline]
    pub fn get_or_insert_info<T: Component>(&mut self) -> &ComponentInfo {
        let id = self.get_or_insert_id::<T>();
        // SAFE: component_info with the given `id` initialized above
        unsafe { self.get_info_unchecked(id) }
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
    /// `id` must be a valid [ComponentId]
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
    pub fn get_or_insert_resource_id<T: Component>(&mut self) -> ComponentId {
        self.get_or_insert_resource_with(TypeId::of::<T>(), TypeInfo::of::<T>)
    }

    #[inline]
    pub fn get_or_insert_non_send_resource_id<T: Any>(&mut self) -> ComponentId {
        self.get_or_insert_resource_with(TypeId::of::<T>(), TypeInfo::of_non_send_and_sync::<T>)
    }

    #[inline]
    fn get_or_insert_resource_with(
        &mut self,
        type_id: TypeId,
        func: impl FnOnce() -> TypeInfo,
    ) -> ComponentId {
        let components = &mut self.components;
        let index = self.resource_indices.entry(type_id).or_insert_with(|| {
            let type_info = func();
            let index = components.len();
            components.push(ComponentInfo::new(ComponentId(index), type_info.into()));
            index
        });

        ComponentId(*index)
    }

    #[inline]
    pub(crate) fn get_or_insert_with(
        &mut self,
        type_id: TypeId,
        func: impl FnOnce() -> TypeInfo,
    ) -> ComponentId {
        let components = &mut self.components;
        let index = self.indices.entry(type_id).or_insert_with(|| {
            let type_info = func();
            let index = components.len();
            components.push(ComponentInfo::new(ComponentId(index), type_info.into()));
            index
        });

        ComponentId(*index)
    }
}

#[derive(Clone, Debug)]
pub struct ComponentTicks {
    pub(crate) added: u32,
    pub(crate) changed: u32,
}

impl ComponentTicks {
    #[inline]
    pub fn is_added(&self, last_change_tick: u32, change_tick: u32) -> bool {
        // The comparison is relative to `change_tick` so that we can detect changes over the whole
        // `u32` range. Comparing directly the ticks would limit to half that due to overflow
        // handling.
        let component_delta = change_tick.wrapping_sub(self.added);
        let system_delta = change_tick.wrapping_sub(last_change_tick);

        component_delta < system_delta
    }

    #[inline]
    pub fn is_changed(&self, last_change_tick: u32, change_tick: u32) -> bool {
        let component_delta = change_tick.wrapping_sub(self.changed);
        let system_delta = change_tick.wrapping_sub(last_change_tick);

        component_delta < system_delta
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
    /// Usually, this is done automatically via the [`DerefMut`](std::ops::DerefMut) implementation
    /// on [`Mut`](crate::world::Mut) or [`ResMut`](crate::system::ResMut) etc.
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
    let tick_delta = change_tick.wrapping_sub(*last_change_tick);
    const MAX_DELTA: u32 = (u32::MAX / 4) * 3;
    // Clamp to max delta
    if tick_delta > MAX_DELTA {
        *last_change_tick = change_tick.wrapping_sub(MAX_DELTA);
    }
}
