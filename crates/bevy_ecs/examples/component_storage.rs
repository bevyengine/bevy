use bevy_ecs::{
    component::{ComponentDescriptor, StorageType},
    prelude::*,
};

// This example shows how to configure the storage of Components.
// A system demonstrates that querying for components is independent of their storage type.
fn main() {
    let mut world = World::new();

    // Store components of type `i32` in a Sparse set
    world
        .register_component(ComponentDescriptor::new::<i32>(StorageType::SparseSet))
        .expect("The component of type i32 is already in use");

    // Components of type i32 will have the above configured Sparse set storage,
    // while f64 components will have the default table storage
    world.spawn().insert(1).insert(0.1);
    world.spawn().insert(2);
    world.spawn().insert(0.2);

    // Setup a schedule and stage to add a system querying for the just spawned entities
    let mut schedule = Schedule::default();
    let mut update = SystemStage::parallel();
    update.add_system(query_entities.system());
    schedule.add_stage("update", update);

    schedule.run(&mut world);
}

// The storage type does not matter for how to query in systems
fn query_entities(entities_with_i32: Query<&i32>, entities_with_f64: Query<&f64>) {
    for value in entities_with_i32.iter() {
        println!("Got entity with i32: {}", value);
    }
    for value in entities_with_f64.iter() {
        println!("Got entity with f64: {}", value);
    }
}
