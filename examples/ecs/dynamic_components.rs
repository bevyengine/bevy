//! This example demonstrates how to create systems and queryies for those systems at runtime
//!
//! The primary use-case for doing so would be allow for integrations with scripting languages,
//! where you do no have the information about what systems exist or what queries they will make.
//!
//! In this example the components are `repr(C)` Rust structs that are spawned from Rust code. To
//! see how to also spawn entities with runtime created Components check out the

use std::{alloc::Layout, time::Duration};

use bevy::prelude::*;
use bevy_app::{RunMode, ScheduleRunnerPlugin, ScheduleRunnerSettings};
use bevy_ecs::{
    DynamicQuery, DynamicSystem, DynamicSystemSettings, EntityBuilder, QueryAccess, TypeInfo,
};

// Set our component IDs. These should probably be hashes of a human-friendly but unique type
// identifier, depending on your scripting implementation needs. These identifiers should have data
// in the last 7 bits. See this comment for more info:
// https://users.rust-lang.org/t/requirements-for-hashes-in-hashbrown-hashmaps/50567/2?u=zicklag.
const POS_COMPONENT_ID: u64 = 242237625853274575;
const VEL_COMPONENT_ID: u64 = 6820197023594215835;

lazy_static::lazy_static! {
    static ref POS_INFO: TypeInfo = TypeInfo::of_external(
        POS_COMPONENT_ID,
        Layout::from_size_align(2, 1).unwrap(),
        |_| (),
    );

    static ref VEL_INFO: TypeInfo = TypeInfo::of_external(
        VEL_COMPONENT_ID,
        Layout::from_size_align(2, 1).unwrap(),
        |_| (),
    );
}

/// Create a system for spawning the scene
fn spawn_scene(world: &mut World, _resources: &mut Resources) {
    // Here we will spawn our dynamically created components

    // For each entity we want to create, we must create an `EntityBuilder` that contains all of
    // that entity's components. We're going to create a couple entities, each with two components,
    // one representing a Position and one representing a Velocity. Each of these will be made up of
    // two bytes for simplicity, one representing the x and y position/velocity.

    // We create our first entity
    let mut builder = EntityBuilder::new();
    // Then we add our "Position component"
    let entity1 = builder
        .add_dynamic(
            // We must specify the component information
            *POS_INFO,
            // And provide the raw byte data data for the component
            vec![
                0, // X position byte
                0, // Y position byte
            ]
            .as_slice(),
        )
        // Next we add our "Velocity component"
        .add_dynamic(
            *VEL_INFO,
            vec![
                0, // X position byte
                1, // Y position byte
            ]
            .as_slice(),
        )
        .build();

    // And let's create another entity
    let mut builder = EntityBuilder::new();
    let entity2 = builder
        .add_dynamic(
            *POS_INFO,
            vec![
                0, // X position byte
                0, // Y position byte
            ]
            .as_slice(),
        )
        .add_dynamic(
            *VEL_INFO,
            vec![
                2, // X position byte
                0, // Y position byte
            ]
            .as_slice(),
        )
        .build();

    // Now we can spawn our entities
    world.spawn(entity1);
    world.spawn(entity2);
}

fn main() {
    // A Dynamic component query which can be constructed at runtime to represent which components
    // we want a dynamic system to access.
    //
    // Notice that the sizes and IDs of the components can be specified at runtime and allow for
    // storage of any data as an array of bytes.
    let mut query = DynamicQuery::default();

    // First we add the info for the components we'll be querying and get their component ids
    let pos_id = query.register_info(*POS_INFO);
    let vel_id = query.register_info(*VEL_INFO);

    // Then we structure our query based on the relationships between the components that we want to
    // query
    query.access = QueryAccess::Union(vec![
        QueryAccess::Read(vel_id, "velocity"),
        QueryAccess::Write(pos_id, "position"),
    ]);

    // Create a dynamic system with the query we constructed
    let pos_vel_system =
        DynamicSystem::new("pos_vel_system".into(), () /* system local state */).settings(
            DynamicSystemSettings {
                queries: vec![query],
                workload: |_state, _resources, queries| {
                    // Print a spacer
                    println!("-----");

                    // Iterate over the query
                    for mut components in queries[0].iter_mut() {
                        // Would panic if we had not set `query.entity = true`
                        let entity = components.entity;
                        let pos_bytes = components.mutable.get_mut(&POS_INFO.id()).unwrap();
                        let vel_bytes = components.immutable.get(&VEL_INFO.id()).unwrap();

                        // Add the X velocity to the X position
                        pos_bytes[0] += vel_bytes[0];
                        // And the same with the Y
                        pos_bytes[1] += vel_bytes[1];

                        // Print out the position and velocity
                        println!(
                            "Entity: {:?}\tPosition: {:?}\tVelocity: {:?}",
                            entity, pos_bytes, vel_bytes
                        );
                    }
                },
                ..Default::default()
            },
        );

    App::build()
        .add_resource(ScheduleRunnerSettings {
            run_mode: RunMode::Loop {
                wait: Some(Duration::from_secs(1)),
            },
        })
        .add_plugin(ScheduleRunnerPlugin::default())
        .add_startup_system(spawn_scene.thread_local_system())
        .add_system(Box::new(pos_vel_system))
        .run();
}
