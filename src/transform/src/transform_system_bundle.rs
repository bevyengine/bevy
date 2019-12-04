use crate::{
    ecs::prelude::*, hierarchy_maintenance_system, local_to_parent_system,
    local_to_world_propagate_system, local_to_world_system,
};

pub fn build(world: &mut World) -> Vec<Box<dyn Schedulable>> {
    let mut all_systems = Vec::with_capacity(5);

    let mut hierarchy_maintenance_systems = hierarchy_maintenance_system::build(world);
    let local_to_parent_system = local_to_parent_system::build(world);
    let local_to_world_system = local_to_world_system::build(world);
    let local_to_world_propagate_system = local_to_world_propagate_system::build(world);

    all_systems.append(&mut hierarchy_maintenance_systems);
    all_systems.push(local_to_parent_system);
    all_systems.push(local_to_world_system);
    all_systems.push(local_to_world_propagate_system);

    all_systems
}
