//! Stress test for large ECS worlds.
//!
//! Running this example:
//!
//! ```
//! cargo run --profile stress-test --example many_components [<num_entities>] [<num_components>] [<num_systems>]
//! ```
//!
//! `num_entities`: The number of entities in the world (must be nonnegative)
//! `num_components`: the number of components in the world (must be at least 10)
//! `num_systems`: the number of systems in the world (must be nonnegative)
//!
//! If no valid number is provided, for each argument there's a reasonable default.

use bevy::{
    diagnostic::{
        DiagnosticPath, DiagnosticsPlugin, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin,
    },
    ecs::{
        component::{ComponentDescriptor, ComponentId, StorageType},
        system::QueryParamBuilder,
        world::FilteredEntityMut,
    },
    log::LogPlugin,
    prelude::{App, In, IntoSystem, Query, Schedule, SystemParamBuilder, Update},
    ptr::OwningPtr,
    MinimalPlugins,
};

use rand::prelude::{Rng, SeedableRng, SliceRandom};
use rand_chacha::ChaCha8Rng;
use std::{alloc::Layout, num::Wrapping};

// A simple system that matches against several components and does some menial calculation to create
// some non-trivial load.
fn base_system(access_components: In<Vec<ComponentId>>, mut query: Query<FilteredEntityMut>) {
    for mut filtered_entity in &mut query {
        // We calculate Faulhaber's formula mod 256 with n = value and p = exponent.
        // See https://en.wikipedia.org/wiki/Faulhaber%27s_formula
        // The time is takes to compute this depends on the number of entities and the values in
        // each entity. This is to ensure that each system takes a different amount of time.
        let mut total: Wrapping<u8> = Wrapping(0);
        let mut exponent: u32 = 1;
        for component_id in &access_components.0 {
            // find the value of the component
            let ptr = filtered_entity.get_by_id(*component_id).unwrap();

            #[expect(unsafe_code)]
            // SAFETY: All components have a u8 layout
            let value: u8 = unsafe { *ptr.deref::<u8>() };

            for i in 0..=value {
                let mut product = Wrapping(1);
                for _ in 1..=exponent {
                    product *= Wrapping(i);
                }
                total += product;
            }
            exponent += 1;
        }

        // we assign this value to all the components we can write to
        for component_id in &access_components.0 {
            if let Some(ptr) = filtered_entity.get_mut_by_id(*component_id) {
                #[expect(unsafe_code)]
                // SAFETY: All components have a u8 layout
                unsafe {
                    let mut value = ptr.with_type::<u8>();
                    *value = total.0;
                }
            }
        }
    }
}

fn stress_test(num_entities: u32, num_components: u32, num_systems: u32) {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut app = App::default();
    let world = app.world_mut();

    // register a bunch of components
    let component_ids: Vec<ComponentId> = (1..=num_components)
        .map(|i| {
            world.register_component_with_descriptor(
                #[allow(unsafe_code)]
                // SAFETY:
                // we don't implement a drop function
                // u8 is Sync and Send
                unsafe {
                    ComponentDescriptor::new_with_layout(
                        format!("Component{}", i).to_string(),
                        StorageType::Table,
                        Layout::new::<u8>(),
                        None,
                        true, // is mutable
                    )
                },
            )
        })
        .collect();

    // fill the schedule with systems
    let mut schedule = Schedule::new(Update);
    for _ in 1..=num_systems {
        let num_access_components = rng.gen_range(1..10);
        let access_components: Vec<ComponentId> = component_ids
            .choose_multiple(&mut rng, num_access_components)
            .copied()
            .collect();
        let system = (QueryParamBuilder::new(|builder| {
            for &access_component in &access_components {
                if rand::random::<bool>() {
                    builder.mut_id(access_component);
                } else {
                    builder.ref_id(access_component);
                }
            }
        }),)
            .build_state(world)
            .build_any_system(base_system);
        schedule.add_systems((move || access_components.clone()).pipe(system));
    }

    // spawn a bunch of entities
    for _ in 1..=num_entities {
        let num_components = rng.gen_range(1..10);
        let components = component_ids.choose_multiple(&mut rng, num_components);

        let mut entity = world.spawn_empty();
        for &component_id in components {
            let value: u8 = rng.gen_range(0..255);
            OwningPtr::make(value, |ptr| {
                #[allow(unsafe_code)]
                // SAFETY:
                // component_id is from the same world
                // value is u8, so ptr is a valid reference for component_id
                unsafe {
                    entity.insert_by_id(component_id, ptr);
                }
            });
        }
    }

    // overwrite Update schedule in the app
    app.add_schedule(schedule);
    app.add_plugins(MinimalPlugins)
        .add_plugins(DiagnosticsPlugin)
        .add_plugins(LogPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(LogDiagnosticsPlugin::filtered(vec![DiagnosticPath::new(
            "fps",
        )]));
    app.run();
}

#[expect(missing_docs)]
pub fn main() {
    const DEFAULT_NUM_ENTITIES: u32 = 50000;
    const DEFAULT_NUM_COMPONENTS: u32 = 1000;
    const DEFAULT_NUM_SYSTEMS: u32 = 800;

    // take input
    let num_entities = std::env::args()
        .nth(1)
        .and_then(|string| string.parse::<u32>().ok())
        .unwrap_or_else(|| {
            println!(
                "No valid number of entities provided, using default {}",
                DEFAULT_NUM_ENTITIES
            );
            DEFAULT_NUM_ENTITIES
        });
    let num_components = std::env::args()
        .nth(2)
        .and_then(|string| string.parse::<u32>().ok())
        .and_then(|n| if n >= 10 { Some(n) } else { None })
        .unwrap_or_else(|| {
            println!(
                "No valid number of components provided (>= 10), using default {}",
                DEFAULT_NUM_COMPONENTS
            );
            DEFAULT_NUM_COMPONENTS
        });
    let num_systems = std::env::args()
        .nth(3)
        .and_then(|string| string.parse::<u32>().ok())
        .unwrap_or_else(|| {
            println!(
                "No valid number of systems provided, using default {}",
                DEFAULT_NUM_SYSTEMS
            );
            DEFAULT_NUM_SYSTEMS
        });

    stress_test(num_entities, num_components, num_systems);
}
