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
        component::{ComponentCloneBehavior, ComponentDescriptor, ComponentId, StorageType},
        system::QueryParamBuilder,
        world::FilteredEntityMut,
    },
    log::LogPlugin,
    platform::collections::HashSet,
    prelude::{App, In, IntoSystem, Query, Schedule, SystemParamBuilder, Update},
    ptr::{OwningPtr, PtrMut},
    MinimalPlugins,
};

use rand::prelude::{IndexedRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::{alloc::Layout, mem::ManuallyDrop, num::Wrapping};

#[expect(unsafe_code, reason = "Reading dynamic components requires unsafe")]
// A simple system that matches against several components and does some menial calculation to create
// some non-trivial load.
fn base_system(access_components: In<Vec<ComponentId>>, mut query: Query<FilteredEntityMut>) {
    #[cfg(feature = "trace")]
    let _span = tracing::info_span!("base_system", components = ?access_components.0, count = query.iter().len()).entered();

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
                // SAFETY: All components have a u8 layout
                unsafe {
                    let mut value = ptr.with_type::<u8>();
                    *value = total.0;
                }
            }
        }
    }
}

#[expect(unsafe_code, reason = "Using dynamic components requires unsafe")]
fn stress_test(num_entities: u32, num_components: u32, num_systems: u32) {
    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut app = App::default();
    let world = app.world_mut();

    // register a bunch of components
    let component_ids: Vec<ComponentId> = (1..=num_components)
        .map(|i| {
            world.register_component_with_descriptor(
                // SAFETY:
                // * We don't implement a drop function
                // * u8 is Sync and Send
                unsafe {
                    ComponentDescriptor::new_with_layout(
                        format!("Component{i}").to_string(),
                        StorageType::Table,
                        Layout::new::<u8>(),
                        None,
                        true, // is mutable
                        ComponentCloneBehavior::Default,
                    )
                },
            )
        })
        .collect();

    // fill the schedule with systems
    let mut schedule = Schedule::new(Update);
    for _ in 1..=num_systems {
        let num_access_components = rng.random_range(1..10);
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
        let num_components = rng.random_range(1..10);
        let components: Vec<ComponentId> = component_ids
            .choose_multiple(&mut rng, num_components)
            .copied()
            .collect();

        let mut entity = world.spawn_empty();
        // We use `ManuallyDrop` here as we need to avoid dropping the u8's when `values` is dropped
        // since ownership of the values is passed to the world in `insert_by_ids`.
        // But we do want to deallocate the memory when values is dropped.
        let mut values: Vec<ManuallyDrop<u8>> = components
            .iter()
            .map(|_id| ManuallyDrop::new(rng.random_range(0..255)))
            .collect();
        let ptrs: Vec<OwningPtr> = values
            .iter_mut()
            .map(|value| {
                // SAFETY:
                // * We don't read/write `values` binding after this and values are `ManuallyDrop`,
                // so we have the right to drop/move the values
                unsafe { PtrMut::from(value).promote() }
            })
            .collect();
        // SAFETY:
        // * component_id's are from the same world
        // * `values` was initialized above, so references are valid
        unsafe {
            entity.insert_by_ids(&components, ptrs.into_iter());
        }
    }

    // overwrite Update schedule in the app
    app.add_schedule(schedule);
    app.add_plugins(MinimalPlugins)
        .add_plugins(DiagnosticsPlugin)
        .add_plugins(LogPlugin::default())
        .add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(LogDiagnosticsPlugin::filtered(HashSet::from_iter([
            DiagnosticPath::new("fps"),
        ])));
    app.run();
}

fn main() {
    const DEFAULT_NUM_ENTITIES: u32 = 50000;
    const DEFAULT_NUM_COMPONENTS: u32 = 1000;
    const DEFAULT_NUM_SYSTEMS: u32 = 800;

    // take input
    let num_entities = std::env::args()
        .nth(1)
        .and_then(|string| string.parse::<u32>().ok())
        .unwrap_or_else(|| {
            println!("No valid number of entities provided, using default {DEFAULT_NUM_ENTITIES}");
            DEFAULT_NUM_ENTITIES
        });
    let num_components = std::env::args()
        .nth(2)
        .and_then(|string| string.parse::<u32>().ok())
        .and_then(|n| if n >= 10 { Some(n) } else { None })
        .unwrap_or_else(|| {
            println!(
                "No valid number of components provided (>= 10), using default {DEFAULT_NUM_COMPONENTS}"
            );
            DEFAULT_NUM_COMPONENTS
        });
    let num_systems = std::env::args()
        .nth(3)
        .and_then(|string| string.parse::<u32>().ok())
        .unwrap_or_else(|| {
            println!("No valid number of systems provided, using default {DEFAULT_NUM_SYSTEMS}");
            DEFAULT_NUM_SYSTEMS
        });

    stress_test(num_entities, num_components, num_systems);
}
