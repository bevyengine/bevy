mod type_info;

pub use type_info::*;

use std::{borrow::Cow, collections::HashMap};

use crate::storage::SparseSetIndex;
use std::{alloc::Layout, any::TypeId};
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

#[derive(Debug)]
pub struct ComponentInfo {
    id: ComponentId,
    descriptor: ComponentDescriptor,
}

impl ComponentInfo {
    #[inline]
    pub fn descriptor(&self) -> &ComponentDescriptor {
        &self.descriptor
    }

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
    pub fn drop(&self) -> unsafe fn(*mut u8) {
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
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, PartialOrd, Ord)]
pub struct ComponentId(usize);

impl SparseSetIndex for ComponentId {
    #[inline]
    fn sparse_set_index(&self) -> usize {
        self.0
    }

    fn get_sparse_set_index(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct ComponentDescriptor {
    name: Cow<'static, str>,
    storage_type: StorageType,
    target_type: TargetType,
    // SAFETY: This must remain private. It must only be set to "true" if this component is actually Send + Sync
    is_send_and_sync: bool,
    type_id: Option<TypeId>,
    layout: Layout,
    drop: unsafe fn(*mut u8),
}

impl ComponentDescriptor {
    // FIXME(Relations) Remove `new` and `new_targeted` methods once we
    // rebase ontop of derive(Component) so that this is moved to the type system :)
    pub fn new<T: Component>(storage_type: StorageType) -> Self {
        Self::new_targeted::<T>(storage_type, TargetType::None)
    }

    pub fn default<T: Component>() -> Self {
        Self::new_targeted::<T>(StorageType::Table, TargetType::None)
    }

    pub fn new_targeted<T: Component>(storage_type: StorageType, target_type: TargetType) -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            storage_type,
            target_type,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: TypeInfo::drop_ptr::<T>,
        }
    }

    pub fn new_non_send_sync<T: 'static>() -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            storage_type: StorageType::Table,
            target_type: TargetType::None,
            is_send_and_sync: false,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: TypeInfo::drop_ptr::<T>,
        }
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
}

impl From<TypeInfo> for ComponentDescriptor {
    fn from(type_info: TypeInfo) -> Self {
        Self {
            name: type_info.type_name().into(),
            storage_type: StorageType::Table,
            target_type: TargetType::None,
            is_send_and_sync: type_info.is_send_and_sync(),
            type_id: Some(type_info.type_id()),
            drop: type_info.drop(),
            layout: type_info.layout(),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum TargetType {
    None,
    Entity,
}

#[derive(Debug, Default)]
pub struct Components {
    infos: Vec<ComponentInfo>,
    // These are only used by bevy. Scripting/dynamic components should
    // use their own hashmap to lookup CustomId -> ComponentId
    component_indices: HashMap<TypeId, ComponentId, fxhash::FxBuildHasher>,
    resource_indices: HashMap<TypeId, ComponentId, fxhash::FxBuildHasher>,
}

#[derive(Debug, Error)]
pub enum RegistrationError {
    #[error("A component of type {name:?} ({type_id:?}) already exists")]
    ComponentAlreadyExists { type_id: TypeId, name: String },
    #[error("A resource of type {name:?} ({type_id:?}) already exists")]
    ResourceAlreadyExists { type_id: TypeId, name: String },
}

impl Components {
    pub fn new_component(
        &mut self,
        layout: ComponentDescriptor,
    ) -> Result<&ComponentInfo, RegistrationError> {
        let id = ComponentId(self.infos.len());
        if self
            .component_indices
            .contains_key(&layout.type_id().unwrap())
        {
            return Err(RegistrationError::ComponentAlreadyExists {
                type_id: layout.type_id().unwrap(),
                name: layout.name.to_string(),
            });
        }
        self.component_indices.insert(layout.type_id().unwrap(), id);
        self.infos.push(ComponentInfo {
            descriptor: layout,
            id,
        });
        Ok(self.infos.last().unwrap())
    }

    pub fn new_resource(
        &mut self,
        layout: ComponentDescriptor,
    ) -> Result<&ComponentInfo, RegistrationError> {
        let id = ComponentId(self.infos.len());
        if self
            .resource_indices
            .contains_key(&layout.type_id().unwrap())
        {
            return Err(RegistrationError::ResourceAlreadyExists {
                type_id: layout.type_id().unwrap(),
                name: layout.name.to_string(),
            });
        }
        self.resource_indices.insert(layout.type_id().unwrap(), id);
        self.infos.push(ComponentInfo {
            descriptor: layout,
            id,
        });
        Ok(self.infos.last().unwrap())
    }

    pub fn info(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.infos.get(id.0)
    }

    pub fn component_info(&self, type_id: TypeId) -> Option<&ComponentInfo> {
        let id = self.component_indices.get(&type_id).copied()?;
        Some(&self.infos[id.0])
    }

    pub fn resource_info(&self, type_id: TypeId) -> Option<&ComponentInfo> {
        let id = self.resource_indices.get(&type_id).copied()?;
        Some(&self.infos[id.0])
    }

    pub fn component_info_or_insert(&mut self, layout: ComponentDescriptor) -> &ComponentInfo {
        match self
            .component_indices
            .get(&layout.type_id().unwrap())
            .copied()
        {
            Some(id) => &self.infos[id.0],
            None => self.new_component(layout).unwrap(),
        }
    }

    pub fn resource_info_or_insert(&mut self, layout: ComponentDescriptor) -> &ComponentInfo {
        match self
            .resource_indices
            .get(&layout.type_id().unwrap())
            .copied()
        {
            Some(id) => &self.infos[id.0],
            None => self.new_resource(layout).unwrap(),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.infos.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.infos.is_empty()
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
