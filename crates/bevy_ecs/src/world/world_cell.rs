use crate::{
    archetype::ArchetypeComponentId,
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
    pub(crate) access: Rc<RefCell<ArchetypeComponentAccess>>,
}

pub(crate) struct ArchetypeComponentAccess {
    access: SparseSet<ArchetypeComponentId, usize>,
}

impl Default for ArchetypeComponentAccess {
    fn default() -> Self {
        Self {
            access: SparseSet::new(),
        }
    }
}

const UNIQUE_ACCESS: usize = 0;
const BASE_ACCESS: usize = 1;
impl ArchetypeComponentAccess {
    const fn new() -> Self {
        Self {
            access: SparseSet::new(),
        }
    }

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

impl<'w> Drop for WorldCell<'w> {
    fn drop(&mut self) {
        let mut access = self.access.borrow_mut();
        // give world ArchetypeComponentAccess back to reuse allocations
        let _ = std::mem::swap(&mut self.world.archetype_component_access, &mut *access);
    }
}

pub struct WorldBorrow<'w, T> {
    value: &'w T,
    archetype_component_id: ArchetypeComponentId,
    access: Rc<RefCell<ArchetypeComponentAccess>>,
}

impl<'w, T> WorldBorrow<'w, T> {
    fn new(
        value: &'w T,
        archetype_component_id: ArchetypeComponentId,
        access: Rc<RefCell<ArchetypeComponentAccess>>,
    ) -> Self {
        if !access.borrow_mut().read(archetype_component_id) {
            panic!(
                "Attempted to immutably access {}, but it is already mutably borrowed",
                std::any::type_name::<T>()
            )
        }
        Self {
            value,
            archetype_component_id,
            access,
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
        let mut access = self.access.borrow_mut();
        access.drop_read(self.archetype_component_id);
    }
}

pub struct WorldBorrowMut<'w, T> {
    value: Mut<'w, T>,
    archetype_component_id: ArchetypeComponentId,
    access: Rc<RefCell<ArchetypeComponentAccess>>,
}

impl<'w, T> WorldBorrowMut<'w, T> {
    fn new(
        value: Mut<'w, T>,
        archetype_component_id: ArchetypeComponentId,
        access: Rc<RefCell<ArchetypeComponentAccess>>,
    ) -> Self {
        if !access.borrow_mut().write(archetype_component_id) {
            panic!(
                "Attempted to mutably access {}, but it is already mutably borrowed",
                std::any::type_name::<T>()
            )
        }
        Self {
            value,
            archetype_component_id,
            access,
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
        let mut access = self.access.borrow_mut();
        access.drop_write(self.archetype_component_id);
    }
}

impl<'w> WorldCell<'w> {
    pub(crate) fn new(world: &'w mut World) -> Self {
        // this is cheap because ArchetypeComponentAccess::new() is const / allocation free
        let access = std::mem::replace(
            &mut world.archetype_component_access,
            ArchetypeComponentAccess::new(),
        );
        // world's ArchetypeComponentAccess is recycled to cut down on allocations
        Self {
            world,
            access: Rc::new(RefCell::new(access)),
        }
    }

    pub fn get_resource<T: Resource>(&self) -> Option<WorldBorrow<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrow::new(
            // SAFE: ComponentId matches TypeId
            unsafe { self.world.get_resource_with_id(component_id)? },
            archetype_component_id,
            self.access.clone(),
        ))
    }

    pub fn get_resource_mut<T: Resource>(&self) -> Option<WorldBorrowMut<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrowMut::new(
            // SAFE: ComponentId matches TypeId and access is checked by WorldBorrowMut
            unsafe {
                self.world
                    .get_resource_unchecked_mut_with_id(component_id)?
            },
            archetype_component_id,
            self.access.clone(),
        ))
    }

    pub fn get_non_send<T: 'static>(&self) -> Option<WorldBorrow<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrow::new(
            // SAFE: ComponentId matches TypeId
            unsafe { self.world.get_non_send_with_id(component_id)? },
            archetype_component_id,
            self.access.clone(),
        ))
    }

    pub fn get_non_send_mut<T: 'static>(&self) -> Option<WorldBorrowMut<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldBorrowMut::new(
            // SAFE: ComponentId matches TypeId and access is checked by WorldBorrowMut
            unsafe {
                self.world
                    .get_non_send_unchecked_mut_with_id(component_id)?
            },
            archetype_component_id,
            self.access.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::BASE_ACCESS;
    use crate::{archetype::ArchetypeId, world::World};
    use std::any::TypeId;

    #[test]
    fn world_cell() {
        let mut world = World::default();
        world.insert_resource(1u32);
        world.insert_resource(1u64);
        let cell = world.cell();
        {
            let mut a = cell.get_resource_mut::<u32>().unwrap();
            assert_eq!(1, *a);
            *a = 2;
        }
        {
            let a = cell.get_resource::<u32>().unwrap();
            assert_eq!(2, *a, "ensure access is dropped");

            let b = cell.get_resource::<u32>().unwrap();
            assert_eq!(
                2, *b,
                "ensure multiple immutable accesses can occur at the same time"
            );
        }
        {
            let a = cell.get_resource_mut::<u32>().unwrap();
            assert_eq!(
                2, *a,
                "ensure both immutable accesses are dropped, enabling a new mutable access"
            );

            let b = cell.get_resource::<u64>().unwrap();
            assert_eq!(
                1, *b,
                "ensure multiple non-conflicting mutable accesses can occur at the same time"
            );
        }
    }

    #[test]
    fn world_access_reused() {
        let mut world = World::default();
        world.insert_resource(1u32);
        {
            let cell = world.cell();
            {
                let mut a = cell.get_resource_mut::<u32>().unwrap();
                assert_eq!(1, *a);
                *a = 2;
            }
        }

        let u32_component_id = world
            .components
            .get_resource_id(TypeId::of::<u32>())
            .unwrap();
        let resource_archetype = world.archetypes.get(ArchetypeId::RESOURCE).unwrap();
        let u32_archetype_component_id = resource_archetype
            .get_archetype_component_id(u32_component_id)
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
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource_mut::<u32>().unwrap();
        let _value_b = cell.get_resource_mut::<u32>().unwrap();
    }

    #[test]
    #[should_panic]
    fn world_cell_ref_and_mut() {
        let mut world = World::default();
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource::<u32>().unwrap();
        let _value_b = cell.get_resource_mut::<u32>().unwrap();
    }

    #[test]
    #[should_panic]
    fn world_cell_mut_and_ref() {
        let mut world = World::default();
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource_mut::<u32>().unwrap();
        let _value_b = cell.get_resource::<u32>().unwrap();
    }

    #[test]
    #[should_panic]
    fn world_cell_ref_and_ref() {
        let mut world = World::default();
        world.insert_resource(1u32);
        let cell = world.cell();
        let _value_a = cell.get_resource_mut::<u32>().unwrap();
        let _value_b = cell.get_resource::<u32>().unwrap();
    }
}
