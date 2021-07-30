use crate::storage::SparseSetIndex;
use std::{
    alloc::Layout,
    any::TypeId,
    borrow::Cow,
    collections::{hash_map::Entry, HashMap},
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

    fn new(id: ComponentId, descriptor: ComponentDescriptor) -> Self {
        ComponentInfo { id, descriptor }
    }
}

#[derive(Debug, Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
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
    // SAFETY: This must remain private. It must only be set to "true" if this component is
    // actually Send + Sync
    is_send_and_sync: bool,
    type_id: Option<TypeId>,
    layout: Layout,
    drop: unsafe fn(*mut u8),
}

impl ComponentDescriptor {
    // SAFETY: The pointer points to a valid value of type `T` and it is safe to drop this value.
    unsafe fn drop_ptr<T>(x: *mut u8) {
        x.cast::<T>().drop_in_place()
    }

    // FIXME(Relations) Remove `new` and `new_targeted` methods once we
    // rebase ontop of derive(Component) so that this is moved to the type system :)
    pub fn new<T: Component>(storage_type: StorageType) -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            storage_type,
            target_type: TargetType::None,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: Self::drop_ptr::<T>,
        }
    }

    pub fn default<T: Component>() -> Self {
        Self::new::<T>(StorageType::Table)
    }

    pub fn new_targeted<T: Component>(storage_type: StorageType) -> Self {
        Self {
            name: std::any::type_name::<T>().into(),
            storage_type,
            target_type: TargetType::Entity,
            is_send_and_sync: true,
            type_id: Some(TypeId::of::<T>()),
            layout: Layout::new::<T>(),
            drop: Self::drop_ptr::<T>,
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
            drop: Self::drop_ptr::<T>,
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
pub enum ComponentsError {
    #[error("A component of type {name:?} ({type_id:?}) already exists")]
    ComponentAlreadyExists {
        type_id: TypeId,
        name: Cow<'static, str>,
    },
    #[error("A resource of type {name:?} ({type_id:?}) already exists")]
    ResourceAlreadyExists {
        type_id: TypeId,
        name: Cow<'static, str>,
    },
}

impl Components {
    #[inline]
    pub fn new_component(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> Result<&ComponentInfo, ComponentsError> {
        let index = self.infos.len();
        if let Some(type_id) = descriptor.type_id {
            let index_entry = self.component_indices.entry(type_id);
            if let Entry::Occupied(_) = index_entry {
                return Err(ComponentsError::ComponentAlreadyExists {
                    type_id,
                    name: descriptor.name,
                });
            }
            self.component_indices.insert(type_id, ComponentId(index));
        }
        self.infos
            .push(ComponentInfo::new(ComponentId(index), descriptor));
        Ok(unsafe { self.infos.get_unchecked(index) })
    }

    #[inline]
    pub fn new_resource(
        &mut self,
        descriptor: ComponentDescriptor,
    ) -> Result<&ComponentInfo, ComponentsError> {
        let index = self.infos.len();
        if let Some(type_id) = descriptor.type_id {
            let index_entry = self.resource_indices.entry(type_id);
            if let Entry::Occupied(_) = index_entry {
                return Err(ComponentsError::ResourceAlreadyExists {
                    type_id,
                    name: descriptor.name,
                });
            }
            self.resource_indices.insert(type_id, ComponentId(index));
        }
        self.infos
            .push(ComponentInfo::new(ComponentId(index), descriptor));
        Ok(unsafe { self.infos.get_unchecked(index) })
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.infos.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.infos.len() == 0
    }

    #[inline]
    pub fn info(&self, id: ComponentId) -> Option<&ComponentInfo> {
        self.infos.get(id.0)
    }

    #[inline]
    /// Safety
    ///
    /// `id` must be a valid id
    pub unsafe fn info_unchecked(&self, id: ComponentId) -> &ComponentInfo {
        debug_assert!(id.0 < self.infos.len());
        self.infos.get_unchecked(id.0)
    }

    //
    #[inline]
    pub fn component_info(&self, type_id: TypeId) -> Option<&ComponentInfo> {
        let id = self.component_indices.get(&type_id).copied()?;
        Some(unsafe { self.infos.get_unchecked(id.0) })
    }

    #[inline]
    pub fn component_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.component_indices.get(&type_id).copied()
    }

    //
    #[inline]
    pub fn resource_info(&self, type_id: TypeId) -> Option<&ComponentInfo> {
        let id = self.resource_indices.get(&type_id).copied()?;
        Some(unsafe { self.infos.get_unchecked(id.0) })
    }

    #[inline]
    pub fn resource_id(&self, type_id: TypeId) -> Option<ComponentId> {
        self.resource_indices.get(&type_id).copied()
    }

    //
    #[inline]
    pub fn component_info_or_insert_from(&mut self, layout: ComponentDescriptor) -> &ComponentInfo {
        match self.component_indices.get(&layout.type_id().unwrap()) {
            Some(&id) => unsafe { self.infos.get_unchecked(id.0) },
            None => self.new_component(layout).unwrap(),
        }
    }

    #[inline]
    pub fn component_info_or_insert<T: Component>(&mut self) -> &ComponentInfo {
        self.component_info_or_insert_from(ComponentDescriptor::default::<T>())
    }

    //
    #[inline]
    pub fn component_id_or_insert_from(&mut self, layout: ComponentDescriptor) -> ComponentId {
        self.component_indices
            .get(&layout.type_id().unwrap())
            .copied()
            .unwrap_or_else(|| self.new_component(layout).unwrap().id)
    }

    #[inline]
    pub fn component_id_or_insert<T: Component>(&mut self) -> ComponentId {
        self.component_id_or_insert_from(ComponentDescriptor::default::<T>())
    }

    //
    #[inline]
    pub fn resource_info_or_insert_from(&mut self, layout: ComponentDescriptor) -> &ComponentInfo {
        match self.resource_indices.get(&layout.type_id().unwrap()) {
            Some(&id) => unsafe { self.infos.get_unchecked(id.0) },
            None => self.new_resource(layout).unwrap(),
        }
    }

    #[inline]
    pub fn resource_info_or_insert<T: Send + Sync + 'static>(&mut self) -> &ComponentInfo {
        self.resource_info_or_insert_from(ComponentDescriptor::default::<T>())
    }

    #[inline]
    pub fn non_send_resource_info_or_insert<T: 'static>(&mut self) -> &ComponentInfo {
        self.resource_info_or_insert_from(ComponentDescriptor::new_non_send_sync::<T>())
    }

    //
    #[inline]
    pub fn resource_id_or_insert_from(&mut self, layout: ComponentDescriptor) -> ComponentId {
        self.resource_indices
            .get(&layout.type_id().unwrap())
            .copied()
            .unwrap_or_else(|| self.new_resource(layout).unwrap().id)
    }

    #[inline]
    pub fn resource_id_or_insert<T: Send + Sync + 'static>(&mut self) -> ComponentId {
        self.resource_id_or_insert_from(ComponentDescriptor::default::<T>())
    }

    #[inline]
    pub fn non_send_resource_id_or_insert<T: 'static>(&mut self) -> ComponentId {
        self.resource_id_or_insert_from(ComponentDescriptor::new_non_send_sync::<T>())
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
