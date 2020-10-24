//! This example demonstrates how to create systems and queries at runtime
//!
//! The primary use-case for doing so would be allow for integrations with scripting languages,
//! where you do no have the information about what systems exist, or what queries they will make,
//! at compile time.
//!
//! In this example the components are Rust structs that are spawned from Rust code. To see how to
//! also spawn entities with runtime created Components check out the `dynamic_components` example.

use std::time::Duration;

use bevy::prelude::*;
use bevy_app::{RunMode, ScheduleRunnerPlugin, ScheduleRunnerSettings};
use bevy_ecs::{DynamicQuery, DynamicSystem, DynamicSystemSettings, QueryAccess, TypeInfo};

lazy_static::lazy_static! {
    static ref POS_INFO: TypeInfo = TypeInfo::of::<Pos>();
    static ref VEL_INFO: TypeInfo = TypeInfo::of::<Vel>();
}

// Define our components

#[derive(Debug, Clone, Copy)]
struct Pos {
    x: f32,
    y: f32,
}

#[derive(Debug, Clone, Copy)]
struct Vel {
    x: f32,
    y: f32,
}

/// Create a system for spawning the scene
fn spawn_scene(world: &mut World, _resources: &mut Resources) {
    #[rustfmt::skip]
    world.spawn_batch(vec![
        (
            Pos {
                x: 0.,
                y: 0.
            },
            Vel {
                x: 0.,
                y: -1.,
            }
        ),
        (
            Pos {
                x: 0.,
                y: 0.
            },
            Vel {
                x: 0.,
                y: 1.,
            }
        ),
        (
            Pos {
                x: 1.,
                y: 1.
            },
            Vel {
                x: -0.5,
                y: 0.5,
            }
        ),
    ]);
}

fn main() {
    // Create a DynamicQuery which can be to outline which components we want a dynamic system to
    // access.
    //
    // Notice that the sizes and IDs of the components must be specified at runtime but this allows
    // for storage of any data type as an array of bytes.
    let mut query = DynamicQuery::default();

    // First we add the info for the components we'll be querying and get their component ids
    let pos_id = query.register_info(*POS_INFO);
    let vel_id = query.register_info(*VEL_INFO);

    // Then we structure our query based on the relationships between the components that we want to
    // query
    query.access = QueryAccess::union(vec![
        QueryAccess::Read(vel_id, "velocity"),
        QueryAccess::Write(pos_id, "position"),
    ]);

    // Create a dynamic system
    let pos_vel_system = DynamicSystem::new(
        "pos_vel_system".into(),
        (), /* system local state, can be any type */
    )
    .settings(
        // Specify the settings for our dynamic system
        DynamicSystemSettings {
            // Specify all of our queries
            queries: vec![
                // In this case we only have one query, but there could be multiple
                query,
            ],
            workload: |_state, _resources, queries| {
                println!("-----");
                // Iterate over the first ( and only ) query and get the component results
                for mut components in queries[0].iter_mut() {
                    let pos_id = POS_INFO.id();
                    let vel_id = VEL_INFO.id();
                    // We reference the slices from our mutable and immutable components vectors. The
                    // indices of the components in the vectors will correspond to the indices that
                    // they were at in the query we created earlier.
                    let pos_bytes = components.mutable.get_mut(&pos_id).unwrap();
                    let vel_bytes = components.immutable.get(&vel_id).unwrap();

                    // Here we have a couple of utility functions to cast the slices back to their
                    // original types.
                    unsafe fn from_slice_mut<T>(s: &mut [u8]) -> &mut T {
                        debug_assert_eq!(std::mem::size_of::<T>(), s.len());
                        &mut *(s.as_mut_ptr() as *mut T)
                    }
                    unsafe fn from_slice<T>(s: &[u8]) -> &T {
                        debug_assert_eq!(std::mem::size_of::<T>(), s.len());
                        &*(s.as_ptr() as *mut T)
                    }

                    // Instead of interacting with the raw bytes of our components, we first cast them to
                    // their Rust structs
                    let mut pos: &mut Pos = unsafe { from_slice_mut(pos_bytes) };
                    let vel: &Vel = unsafe { from_slice(vel_bytes) };

                    // Now we can operate on our components like we would normally in Rust
                    pos.x += vel.x;
                    pos.y += vel.y;

                    println!("{:?}\t\t{:?}", pos, vel);
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
