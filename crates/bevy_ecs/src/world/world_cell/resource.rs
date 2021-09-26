use crate::{
    archetype::ArchetypeComponentId,
    storage::SparseSet,
    system::Resource,
    world::{Mut, WorldCell, WorldCellState},
};
use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

pub(crate) struct ArchetypeComponentAccess {
    access: SparseSet<ArchetypeComponentId, u32>,
}

const UNIQUE_ACCESS: u32 = 0;
const BASE_ACCESS: u32 = 1;
impl ArchetypeComponentAccess {
    pub(super) const fn new() -> Self {
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

pub struct WorldCellRes<'w, T> {
    value: &'w T,
    archetype_component_id: ArchetypeComponentId,
    state: &'w WorldCellState,
}

impl<'w, T> WorldCellRes<'w, T> {
    fn new(
        value: &'w T,
        archetype_component_id: ArchetypeComponentId,
        state: &'w WorldCellState,
    ) -> Self {
        if !state
            .resource_access
            .borrow_mut()
            .read(archetype_component_id)
        {
            panic!(
                "Attempted to immutably access {}, but it is already mutably borrowed",
                std::any::type_name::<T>()
            )
        }
        Self {
            value,
            archetype_component_id,
            state,
        }
    }
}

impl<'w, T> Deref for WorldCellRes<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<'w, T> Drop for WorldCellRes<'w, T> {
    fn drop(&mut self) {
        let mut access = self.state.resource_access.borrow_mut();
        access.drop_read(self.archetype_component_id);
    }
}

pub struct WorldCellResMut<'w, T> {
    value: Mut<'w, T>,
    archetype_component_id: ArchetypeComponentId,
    state: &'w WorldCellState,
}

impl<'w, T> WorldCellResMut<'w, T> {
    fn new(
        value: Mut<'w, T>,
        archetype_component_id: ArchetypeComponentId,
        state: &'w WorldCellState,
    ) -> Self {
        if !state
            .resource_access
            .borrow_mut()
            .write(archetype_component_id)
        {
            panic!(
                "Attempted to mutably access {}, but it is already mutably borrowed",
                std::any::type_name::<T>()
            )
        }
        Self {
            value,
            archetype_component_id,
            state,
        }
    }
}

impl<'w, T> Deref for WorldCellResMut<'w, T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.value.deref()
    }
}

impl<'w, T> DerefMut for WorldCellResMut<'w, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.value
    }
}

impl<'w, T> Drop for WorldCellResMut<'w, T> {
    fn drop(&mut self) {
        let mut access = self.state.resource_access.borrow_mut();
        access.drop_write(self.archetype_component_id);
    }
}

impl<'w> WorldCell<'w> {
    pub fn get_resource<T: Resource>(&self) -> Option<WorldCellRes<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellRes::new(
            // SAFE: ComponentId matches TypeId
            unsafe { self.world.get_resource_with_id(component_id)? },
            archetype_component_id,
            &self.state,
        ))
    }
    pub fn get_resource_mut<T: Resource>(&self) -> Option<WorldCellResMut<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellResMut::new(
            // SAFE: ComponentId matches TypeId and access is checked by WorldCellResMut
            unsafe {
                self.world
                    .get_resource_unchecked_mut_with_id(component_id)?
            },
            archetype_component_id,
            &self.state,
        ))
    }

    pub fn get_non_send<T: 'static>(&self) -> Option<WorldCellRes<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellRes::new(
            // SAFE: ComponentId matches TypeId
            unsafe { self.world.get_non_send_with_id(component_id)? },
            archetype_component_id,
            &self.state,
        ))
    }

    pub fn get_non_send_mut<T: 'static>(&self) -> Option<WorldCellResMut<'_, T>> {
        let component_id = self.world.components.get_resource_id(TypeId::of::<T>())?;
        let resource_archetype = self.world.archetypes.resource();
        let archetype_component_id = resource_archetype.get_archetype_component_id(component_id)?;
        Some(WorldCellResMut::new(
            // SAFE: ComponentId matches TypeId and access is checked by WorldCellResMut
            unsafe {
                self.world
                    .get_non_send_unchecked_mut_with_id(component_id)?
            },
            archetype_component_id,
            &self.state,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::BASE_ACCESS;
    use crate::{archetype::ArchetypeId, world::World};
    use std::any::TypeId;

    #[test]
    fn world_cell_res_access() {
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
        assert_eq!(
            world.world_cell_state.resource_access.borrow().access.len(),
            1
        );
        assert_eq!(
            world
                .world_cell_state
                .resource_access
                .borrow()
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
