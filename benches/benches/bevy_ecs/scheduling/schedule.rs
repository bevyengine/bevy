use bevy_app::{App, Update};

use bevy_ecs::{
    component::{ComponentDescriptor, ComponentId, StorageType},
    prelude::*,
    system::QueryParamBuilder,
    world::FilteredEntityMut,
};
use bevy_ptr::OwningPtr;
use criterion::Criterion;
use rand::prelude::SeedableRng;
use rand::prelude::SliceRandom;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::alloc::Layout;
use std::num::Wrapping;

pub fn schedule(c: &mut Criterion) {
    #[derive(Component)]
    struct A(f32);
    #[derive(Component)]
    struct B(f32);
    #[derive(Component)]
    struct C(f32);
    #[derive(Component)]
    struct D(f32);
    #[derive(Component)]
    struct E(f32);

    fn ab(mut query: Query<(&mut A, &mut B)>) {
        query.iter_mut().for_each(|(mut a, mut b)| {
            core::mem::swap(&mut a.0, &mut b.0);
        });
    }

    fn cd(mut query: Query<(&mut C, &mut D)>) {
        query.iter_mut().for_each(|(mut c, mut d)| {
            core::mem::swap(&mut c.0, &mut d.0);
        });
    }

    fn ce(mut query: Query<(&mut C, &mut E)>) {
        query.iter_mut().for_each(|(mut c, mut e)| {
            core::mem::swap(&mut c.0, &mut e.0);
        });
    }

    let mut group = c.benchmark_group("schedule");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("base", |b| {
        let mut world = World::default();

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0), D(0.0))));

        world.spawn_batch((0..10000).map(|_| (A(0.0), B(0.0), C(0.0), E(0.0))));

        let mut schedule = Schedule::default();
        schedule.add_systems((ab, cd, ce));
        schedule.run(&mut world);

        b.iter(move || schedule.run(&mut world));
    });
    group.finish();
}

pub fn build_schedule(criterion: &mut Criterion) {
    // empty system
    fn empty_system() {}

    // Use multiple different kinds of label to ensure that dynamic dispatch
    // doesn't somehow get optimized away.
    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct NumSet(usize);

    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct DummySet;

    let mut group = criterion.benchmark_group("build_schedule");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(15));

    // Method: generate a set of `graph_size` systems which have a One True Ordering.
    // Add system to the schedule with full constraints. Hopefully this should be maximally
    // difficult for bevy to figure out.
    let labels: Vec<_> = (0..1000).map(|i| NumSet(i)).collect();

    // Benchmark graphs of different sizes.
    for graph_size in [100, 500, 1000] {
        // Basic benchmark without constraints.
        group.bench_function(format!("{graph_size}_schedule_noconstraints"), |bencher| {
            bencher.iter(|| {
                let mut app = App::new();
                for _ in 0..graph_size {
                    app.add_systems(Update, empty_system);
                }
                app.update();
            });
        });

        // Benchmark with constraints.
        group.bench_function(format!("{graph_size}_schedule"), |bencher| {
            bencher.iter(|| {
                let mut app = App::new();
                app.add_systems(Update, empty_system.in_set(DummySet));

                // Build a fully-connected dependency graph describing the One True Ordering.
                // Not particularly realistic but this can be refined later.
                for i in 0..graph_size {
                    let mut sys = empty_system.in_set(labels[i]).before(DummySet);
                    for label in labels.iter().take(i) {
                        sys = sys.after(*label);
                    }
                    for label in &labels[i + 1..graph_size] {
                        sys = sys.before(*label);
                    }
                    app.add_systems(Update, sys);
                }
                // Run the app for a single frame.
                // This is necessary since dependency resolution does not occur until the game runs.
                // FIXME: Running the game clutters up the benchmarks, so ideally we'd be
                // able to benchmark the dependency resolution directly.
                app.update();
            });
        });
    }

    group.finish();
}

pub fn empty_schedule_run(criterion: &mut Criterion) {
    let mut app = bevy_app::App::default();

    let mut group = criterion.benchmark_group("run_empty_schedule");

    let mut schedule = Schedule::default();
    schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::SingleThreaded);
    group.bench_function("SingleThreaded", |bencher| {
        bencher.iter(|| schedule.run(app.world_mut()));
    });

    let mut schedule = Schedule::default();
    schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::MultiThreaded);
    group.bench_function("MultiThreaded", |bencher| {
        bencher.iter(|| schedule.run(app.world_mut()));
    });

    let mut schedule = Schedule::default();
    schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::Simple);
    group.bench_function("Simple", |bencher| {
        bencher.iter(|| schedule.run(app.world_mut()));
    });
    group.finish();
}

fn base_system(mut query: Query<FilteredEntityMut>) {
    for filtered_entity in &mut query {
        // we calculate Faulhaber's formula (https://en.wikipedia.org/wiki/Faulhaber%27s_formula) mod 256
        // with n = value and p = exponent for each entity.
        // The time is takes to compute this is dependant on the number of entities in the query and
        // the values in each entity. This is to ensure that the running times between systems are varied.
        let mut total: Wrapping<u8> = Wrapping(0);
        let mut exponent: u32 = 1;
        for component_id in filtered_entity.access().component_reads_and_writes().0 {
            // find the value of the component
            let ptr = filtered_entity.get_by_id(component_id).unwrap();
            // SAFETY: All components have a u8 layout.
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
        for component_id in filtered_entity.access().component_reads_and_writes().0 {
            let ptr = filtered_entity.get_by_id(component_id).unwrap();
            if filtered_entity.access().has_component_write(component_id) {
                // SAFETY:
                // We have exclusive access so the pointer is unique
                // All components have a u8 layout
                unsafe {
                    let value = ptr.assert_unique().deref_mut::<u8>();
                    *value = total.0;
                }
            }
        }
    }
}

// A benchmark that tests running many systems with a lot of components.
// This is mostly intended to test how quickly two systems can figure out how
// they are in conflict via Access<T>.get_conflicts(other: Access<T>)
fn many_components_and_systems(criterion: &mut Criterion) {
    let num_components = 2000;
    let num_systems = 4000;
    let num_entities = 10000;

    let mut rng = ChaCha8Rng::seed_from_u64(42);
    let mut world = World::default();

    // register a bunch of components
    let component_ids: Vec<ComponentId> = (1..=num_components)
        .map(|i| {
            world.register_component_with_descriptor(unsafe {
                ComponentDescriptor::new_with_layout(
                    format!("Component{}", i).to_string(),
                    StorageType::Table,
                    Layout::new::<u8>(),
                    None,
                )
            })
        })
        .collect();

    // fill the schedule with systems
    let mut schedule = Schedule::default();
    for _ in 1..=num_systems {
        let num_access_components = rng.gen_range(1..10);
        let access_components = component_ids.choose_multiple(&mut rng, num_access_components);
        let system = (QueryParamBuilder::new(|builder| {
            for &access_component in access_components {
                if rand::random::<bool>() {
                    builder.mut_id(access_component);
                } else {
                    builder.ref_id(access_component);
                }
            }
        }),)
            .build_state(&mut world)
            .build_system(base_system);
        schedule.add_systems(system);
    }

    // spawn a bunch of entities
    for _ in 1..=num_entities {
        let num_components = rng.gen_range(1..10);
        let components = component_ids.choose_multiple(&mut rng, num_components);

        let mut entity = world.spawn_empty();
        for &component_id in components {
            OwningPtr::make(rng.gen_range(0..255), |ptr| unsafe {
                entity.insert_by_id(component_id, ptr);
            });
        }
    }

    let mut group = criterion.benchmark_group("run_large_schedule");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(15));
    group.bench_function("large_schedule", |bencher| {
        bencher.iter(|| {
            schedule.run(&mut world);
        });
    });
}
