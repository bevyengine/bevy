use bevy_utils::HashMap;

use crate::{
    archetype::ArchetypeComponentId,
    component::ComponentId,
    entity::Entity,
    prelude::Component,
    storage::SparseSet,
    system::Resource,
    world::{Mut, World},
};
use std::{
    any::TypeId,
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
};

/// Exposes safe mutable access to multiple resources at a time in a World. Attempting to access
/// World in a way that violates Rust's mutability rules will panic thanks to runtime checks.
pub struct WorldCell<'w> {
    pub(crate) world: &'w mut World,
    pub(crate) resource_access: Rc<RefCell<ArchetypeComponentAccess>>,
    pub(crate) component_access: Rc<RefCell<EntityComponentAccess>>,
}

#[derive(Default)]
pub(crate) struct ArchetypeComponentAccess {
    access: SparseSet<ArchetypeComponentId, usize>,
}

const UNIQUE_ACCESS: usize = 0;
const BASE_ACCESS: usize = 1;
impl ArchetypeComponentAccess {
    fn read(&mut self, id: ArchetypeComponentId) -> bool {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        if *id_access == UNIQUE_ACCESS {
            false
        } else {
            *id_access += 1;
            true
        }
    }

    fn drop_read(&mut self, id: ArchetypeComponentId) {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        *id_access -= 1;
    }

    fn write(&mut self, id: ArchetypeComponentId) -> bool {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        if *id_access == BASE_ACCESS {
            *id_access = UNIQUE_ACCESS;
            true
        } else {
            false
        }
    }

    fn drop_write(&mut self, id: ArchetypeComponentId) {
        let id_access = self.access.get_or_insert_with(id, || BASE_ACCESS);
        *id_access = BASE_ACCESS;
    }
}

#[derive(Default)]
pub(crate) struct EntityComponentAccess {
    access: HashMap<(Entity, ComponentId), usize>,
}

impl EntityComponentAccess {
    fn read(&mut self, id: (Entity, ComponentId)) -> bool {
        let id_access = self.access.entry(id).or_insert(BASE_ACCESS);
        if *id_access == UNIQUE_ACCESS {
            false
        } else {
            *id_access += 1;
            true
        }
    }

    fn drop_read(&mut self, id: (Entity, ComponentId)) {
        let id_access = self.access.entry(id).or_insert(BASE_ACCESS);
        *id_access -= 1;
    }

    fn write(&mut self, id: (Entity, ComponentId)) -> bool {
        let id_access = self.access.entry(id).or_insert(BASE_ACCESS);
        if *id_access == BASE_ACCESS {
            *id_access = UNIQUE_ACCESS;
            true
        } else {
            false
        }
    }

    fn drop_write(&mut self, id: (Entity, ComponentId)) {
        let id_access = self.access.entry(id).or_insert(BASE_ACCESS);
        *id_access = BASE_ACCESS;
    }
}

impl<'w> Drop for WorldCell<'w> {
    fn drop(&mut self) {
        // give world {Archetype,Entity}ComponentAccess back to reuse allocations
        let mut resource_access = self.resource_access.borrow_mut();
        std::mem::swap(
            &mut self.world.archetype_component_access,
            &mut *resource_access,
        );
        let mut component_access = self.component_access.borrow_mut();
        std::mem::swap(
            &mut self.world.entity_component_access,
            &mut *component_access,
        );
    }
}

enum WorldCellId {
    Resource(ArchetypeComponentId),
    Component((Entity, ComponentId)),
}
impl From<ArchetypeComponentId> for WorldCellId {
    fn from(id: ArchetypeComponentId) -> Self {
        WorldCellId::Resource(id)
    }
}

pub struct WorldBorrow<'w, T> {
    value: &'w T,
    id: WorldCellId,
    resource_access: Rc<RefCell<ArchetypeComponentAccess>>,
    component_access: Rc<RefCell<EntityComponentAccess>>,
}

impl<'w, T> WorldBorrow<'w, T> {
    fn new(
        value: &'w T,
        id: WorldCellId,
        resource_access: Rc<RefCell<ArchetypeComponentAccess>>,
        component_access: Rc<RefCell<EntityComponentAccess>>,
    ) -> Self {
        match id {
            WorldCellId::Resource(archetype_component_id) => {
                assert!(
                    resource_access.borrow_mut().read(archetype_component_id),
                    "Attempted to immutably access {}, but it is already mutably borrowed",
                    std::any::type_name::<T>(),
                );
            }
            WorldCellId::Component((entity, component_id)) => {
                assert!(
                    component_access.borrow_mut().read((entity, component_id)),
                    "Attempted to immutably access component {} on entity {:?}, but it is already mutably borrowed",
                    std::any::type_name::<T>(),
                    entity,
                );
            }
        }
        Self {
            value,
            id,
            resource_access,
            component_access,
        }
    }
}

impl<'w, T> Deref for WorldBorrow<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> Drop for WorldBorrow<'w, T> {
    fn drop(&mut self) {
        match self.id {
            WorldCellId::Resource(id) => self.resource_access.borrow_mut().drop_read(id),
            WorldCellId::Component(id) => self.component_access.borrow_mut().drop_read(id),
        }
    }
}

pub struct WorldBorrowMut<'w, T> {
    value: Mut<'w, T>,
    id: WorldCellId,
    resource_access: Rc<RefCell<ArchetypeComponentAccess>>,
    component_access: Rc<RefCell<EntityComponentAccess>>,
}

impl<'w, T> WorldBorrowMut<'w, T> {
    fn new(
        value: Mut<'w, T>,
        id: WorldCellId,
        resource_access: Rc<RefCell<ArchetypeComponentAccess>>,
        component_access: Rc<RefCell<EntityComponentAccess>>,
    ) -> Self {
        match id {
            WorldCellId::Resource(archetype_component_id) => {
                assert!(
                    resource_access.borrow_mut().write(archetype_component_id),
                    "Attempted to mutably access {}, but it is already mutably borrowed",
                    std::any::type_name::<T>(),
                );
            }
            WorldCellId::Component((entity, component)) => {
                assert!(
                    component_access.borrow_mut().write((entity, component)),
                    "Attempted to mutably access component {} at entity {:?}, but it is already mutably borrowed",
                    std::any::type_name::<T>(),
                    entity
                );
            }
        }
        Self {
            value,
            id,
            resource_access,
            component_access,
        }
    }
}

impl<'w, T> Deref for WorldBorrowMut<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}

impl<'w, T> DerefMut for WorldBorrowMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.value
    }
}

impl<'w, T> Drop for WorldBorrowMut<'w, T> {
    fn drop(&mut self) {
        match self.id {
            WorldCellId::Resource(id) => {
                self.resource_access.borrow_mut().drop_write(id);
            }
            WorldCellId::Component(id) => {
                self.component_access.borrow_mut().drop_write(id);
            }
        }
    }
}

impl<'w> WorldCell<'w> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        // this is cheap because ArchetypeComponentAccess::new() is const / allocation free
        let resource_access = std::mem::take(&mut world.archetype_component_access);
        let component_access = std::mem::take(&mut world.entity_component_access);
        // world's ArchetypeComponentAccess is recycled to cut down on allocations
        Self {
            world,
            resource_access: Rc::new(RefCell::new(resource_access)),
            component_access: Rc::new(RefCell::new(component_access)),
        }
    }

    /// Gets a reference to the resource of the given type
    pub fn get_resource<T: Resource>(&self) -> Option<WorldBorrow<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrow::new(
            // SAFETY: ComponentId matches TypeId
            unsafe { self.world.get_resource_with_id(component_id)? },
            archetype_component_id.into(),
            self.resource_access.clone(),
            self.component_access.clone(),
        ))
    }

    /// Gets a reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist. Use [`get_resource`](WorldCell::get_resource) instead
    /// if you want to handle this case.
    pub fn resource<T: Resource>(&self) -> WorldBorrow<'_, T> {
        match self.get_resource() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_resource` / `app.init_resource`? 
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Gets a mutable reference to the resource of the given type
    pub fn get_resource_mut<T: Resource>(&self) -> Option<WorldBorrowMut<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrowMut::new(
            // SAFETY: ComponentId matches TypeId and access is checked by WorldBorrowMut
            unsafe {
                self.world
                    .get_resource_unchecked_mut_with_id(component_id)?
            },
            archetype_component_id.into(),
            self.resource_access.clone(),
            self.component_access.clone(),
        ))
    }

    /// Gets a mutable reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist. Use [`get_resource_mut`](WorldCell::get_resource_mut)
    /// instead if you want to handle this case.
    pub fn resource_mut<T: Resource>(&self) -> WorldBorrowMut<'_, T> {
        match self.get_resource_mut() {
            Some(x) => x,
            None => panic!(
                "Requested resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_resource` / `app.init_resource`? 
                Resources are also implicitly added via `app.add_event`,
                and can be added by plugins.",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Gets an immutable reference to the non-send resource of the given type, if it exists.
    pub fn get_non_send_resource<T: 'static>(&self) -> Option<WorldBorrow<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrow::new(
            // SAFETY: ComponentId matches TypeId
            unsafe { self.world.get_non_send_with_id(component_id)? },
            archetype_component_id.into(),
            self.resource_access.clone(),
            self.component_access.clone(),
        ))
    }

    /// Gets an immutable reference to the non-send resource of the given type, if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist. Use
    /// [`get_non_send_resource`](WorldCell::get_non_send_resource) instead if you want to handle
    /// this case.
    pub fn non_send_resource<T: 'static>(&self) -> WorldBorrow<'_, T> {
        match self.get_non_send_resource() {
            Some(x) => x,
            None => panic!(
                "Requested non-send resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_non_send_resource` / `app.init_non_send_resource`? 
                Non-send resources can also be be added by plugins.",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    pub fn get_non_send_resource_mut<T: 'static>(&self) -> Option<WorldBorrowMut<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrowMut::new(
            // SAFETY: ComponentId matches TypeId and access is checked by WorldBorrowMut
            unsafe {
                self.world
                    .get_non_send_unchecked_mut_with_id(component_id)?
            },
            archetype_component_id.into(),
            self.resource_access.clone(),
            self.component_access.clone(),
        ))
    }

    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist. Use
    /// [`get_non_send_resource_mut`](WorldCell::get_non_send_resource_mut) instead if you want to
    /// handle this case.
    pub fn non_send_resource_mut<T: 'static>(&self) -> WorldBorrowMut<'_, T> {
        match self.get_non_send_resource_mut() {
            Some(x) => x,
            None => panic!(
                "Requested non-send resource {} does not exist in the `World`. 
                Did you forget to add it using `app.insert_non_send_resource` / `app.init_non_send_resource`? 
                Non-send resources can also be be added by plugins.",
                std::any::type_name::<T>()
            ),
        }
    }

    /// Gets a reference to the component of the given type
    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<WorldBorrow<'_, T>> {
        let component_id = self.world.components.get_id(TypeId::of::<T>())?;
        Some(WorldBorrow::new(
            // SAFETY: ComponentId matches TypeId
            self.world.get(entity)?,
            WorldCellId::Component((entity, component_id)),
            self.resource_access.clone(),
            self.component_access.clone(),
        ))
    }
    /// Gets a mutable reference to the component of the given type
    pub fn get_component_mut<T: Component>(&self, entity: Entity) -> Option<WorldBorrowMut<'_, T>> {
        let component_id = self.world.components.get_id(TypeId::of::<T>())?;
        let last_change_tick = self.world.last_change_tick();
        let change_tick = self.world.read_change_tick();
        Some(WorldBorrowMut::new(
            // SAFETY: ComponentId matches TypeId
            unsafe {
                self.world
                    .get_entity(entity)?
                    .get_unchecked_mut(last_change_tick, change_tick)?
            },
            WorldCellId::Component((entity, component_id)),
            self.resource_access.clone(),
            self.component_access.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::BASE_ACCESS;
    use crate as bevy_ecs;
    use crate::prelude::Component;
    use crate::{archetype::ArchetypeId, system::Resource, world::World};
    use std::any::TypeId;

    #[derive(Component, Resource)]
    struct A(u32);

    #[derive(Component, Resource)]
    struct B(u64);

    #[test]
    fn world_cell() {
        let mut world = World::default();
        world.insert_resource(A(1));
        world.insert_resource(B(1));
        let cell = world.cell();
        {
            let mut a = cell.resource_mut::<A>();
            assert_eq!(1, a.0);
            a.0 = 2;
        }
        {
            let a = cell.resource::<A>();
            assert_eq!(2, a.0, "ensure access is dropped");

            let a2 = cell.resource::<A>();
            assert_eq!(
                2, a2.0,
                "ensure multiple immutable accesses can occur at the same time"
            );
        }
        {
            let a = cell.resource_mut::<A>();
            assert_eq!(
                2, a.0,
                "ensure both immutable accesses are dropped, enabling a new mutable access"
            );

            let b = cell.resource::<B>();
            assert_eq!(
                1, b.0,
                "ensure multiple non-conflicting mutable accesses can occur at the same time"
            );
        }
    }

    #[test]
    fn world_access_reused() {
        let mut world = World::default();
        world.insert_resource(A(1));
        {
            let cell = world.cell();
            {
                let mut a = cell.resource_mut::<A>();
                assert_eq!(1, a.0);
                a.0 = 2;
            }
        }

        let resource_id = world.components.get_resource_id(TypeId::of::<A>()).unwrap();
        let resource_archetype = world.archetypes.get(ArchetypeId::RESOURCE).unwrap();
        let u32_archetype_component_id = resource_archetype
            .get_archetype_component_id(resource_id)
            .unwrap();
        assert_eq!(world.archetype_component_access.access.len(), 1);
        assert_eq!(
            world
                .archetype_component_access
                .access
                .get(u32_archetype_component_id),
            Some(&BASE_ACCESS),
            "reused access count is 'base'"
        );
    }

    #[test]
    #[should_panic]
    fn world_cell_double_mut() {
        let mut world = World::default();
        world.insert_resource(A(1));
        let cell = world.cell();
        let _value_a = cell.resource_mut::<A>();
        let _value_b = cell.resource_mut::<A>();
    }

    #[test]
    #[should_panic]
    fn world_cell_ref_and_mut() {
        let mut world = World::default();
        world.insert_resource(A(1));
        let cell = world.cell();
        let _value_a = cell.resource::<A>();
        let _value_b = cell.resource_mut::<A>();
    }

    #[test]
    #[should_panic]
    fn world_cell_mut_and_ref() {
        let mut world = World::default();
        world.insert_resource(A(1));
        let cell = world.cell();
        let _value_a = cell.resource_mut::<A>();
        let _value_b = cell.resource::<A>();
    }

    #[test]
    fn world_cell_ref_and_ref() {
        let mut world = World::default();
        world.insert_resource(A(1));
        let cell = world.cell();
        let _value_a = cell.resource::<A>();
        let _value_b = cell.resource::<A>();
    }

    #[test]
    #[should_panic]
    fn world_cell_component_mut_and_ref() {
        let mut world = World::default();
        let entity = world.spawn().insert(A(1)).id();
        let cell = world.cell();
        let _value_a = cell.get_component_mut::<A>(entity).unwrap();
        let _value_b = cell.get_component::<A>(entity).unwrap();
    }

    #[test]
    fn world_cell_component_ref_and_ref() {
        let mut world = World::default();
        let entity = world.spawn().insert(A(1)).id();
        let cell = world.cell();
        let _value_a = cell.get_component::<A>(entity).unwrap();
        let _value_b = cell.get_component::<A>(entity).unwrap();
    }

    #[test]
    fn world_cell_component_mut_and_ref_different_entities() {
        let mut world = World::default();
        let entity_1 = world.spawn().insert(A(1)).id();
        let entity_2 = world.spawn().insert(A(1)).id();
        let cell = world.cell();
        let _value_1 = cell.get_component_mut::<A>(entity_1).unwrap();
        let _value_2 = cell.get_component::<A>(entity_2).unwrap();
    }
}
