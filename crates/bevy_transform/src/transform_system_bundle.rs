use crate::{
    ecs::prelude::*, hierarchy_maintenance_system, local_to_parent_system,
    transform_propagate_system, transform_system,
};

pub fn build(world: &mut World) -> Vec<Box<dyn Schedulable>> {
    let mut all_systems = Vec::with_capacity(5);

    let mut hierarchy_maintenance_systems = hierarchy_maintenance_system::build(world);
    let local_to_parent_system = local_to_parent_system::build(world);
    let transform_system = transform_system::build(world);
    let transform_propagate_system = transform_propagate_system::build(world);

    all_systems.append(&mut hierarchy_maintenance_systems);
    all_systems.push(local_to_parent_system);
    all_systems.push(transform_system);
    all_systems.push(transform_propagate_system);

    all_systems
}
