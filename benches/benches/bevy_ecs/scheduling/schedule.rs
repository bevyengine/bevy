use bevy_app::{App, Update};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::{MultiThreadedExecutor, SingleThreadedExecutor, WorkStealingExecutor};
use criterion::{BatchSize, Criterion};
use std::{hint::black_box, rc::Rc};

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
    let labels: Vec<_> = (0..1000).map(NumSet).collect();

    // Benchmark graphs of different sizes.
    for graph_size in [100, 500, 1000] {
        // Basic benchmark without constraints.
        group.bench_function(format!("{graph_size}_schedule_no_constraints"), |bencher| {
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
    let mut app = App::default();

    let mut group = criterion.benchmark_group("run_empty_schedule");

    let mut schedule = Schedule::default();
    schedule.set_executor(SingleThreadedExecutor::new());
    group.bench_function("SingleThreaded", |bencher| {
        bencher.iter(|| schedule.run(app.world_mut()));
    });

    let mut schedule = Schedule::default();
    schedule.set_executor(MultiThreadedExecutor::new());
    group.bench_function("MultiThreaded", |bencher| {
        bencher.iter(|| schedule.run(app.world_mut()));
    });

    let mut schedule = Schedule::default();
    schedule.set_executor(WorkStealingExecutor::new());
    group.bench_function("WorkStealing", |bencher| {
        bencher.iter(|| schedule.run(app.world_mut()));
    });

    group.finish();
}

pub fn compile_schedule_only(criterion: &mut Criterion) {
    fn empty_system() {}

    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct ChainSet(usize);

    let mut group = criterion.benchmark_group("compile_schedule_only");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(8));

    for system_count in [32usize, 128, 512] {
        group.bench_function(format!("{system_count}_systems"), |bencher| {
            bencher.iter_batched(
                || {
                    let world = World::new();
                    let mut schedule = Schedule::default();
                    let labels: Vec<_> = (0..system_count).map(ChainSet).collect();
                    for index in 0..system_count {
                        let mut system = empty_system.in_set(labels[index]);
                        if index > 0 {
                            system = system.after(labels[index - 1]);
                        }
                        schedule.add_systems(system);
                    }
                    (world, schedule)
                },
                |(mut world, schedule)| {
                    let compiled = schedule.compile(&mut world).unwrap();
                    black_box(compiled.systems_len());
                },
                BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

pub fn compiled_schedule_run(criterion: &mut Criterion) {
    fn writer(mut commands: Commands) {
        commands.queue(|world: &mut World| {
            let mut value = world.resource_mut::<Counter>();
            value.0 = value.0.wrapping_add(1);
        });
    }

    fn reader(_counter: Option<Res<Counter>>) {}

    #[derive(Resource, Default)]
    struct Counter(u32);

    let mut group = criterion.benchmark_group("compiled_schedule_run");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    group.bench_function("base", |bencher| {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();
        schedule.add_systems((writer, reader).chain());
        let mut compiled = schedule.compile(&mut world).unwrap();

        bencher.iter(|| compiled.run(&mut world));
    });
    group.finish();
}

pub fn dependency_chain_run(criterion: &mut Criterion) {
    fn empty_system() {}

    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct ChainSet(usize);

    let mut group = criterion.benchmark_group("dependency_chain_run");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for system_count in [16usize, 64, 256] {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        let labels: Vec<_> = (0..system_count).map(ChainSet).collect();
        for index in 0..system_count {
            let mut system = empty_system.in_set(labels[index]);
            if index > 0 {
                system = system.after(labels[index - 1]);
            }
            schedule.add_systems(system);
        }
        schedule.run(&mut world);

        group.bench_function(format!("{system_count}_systems"), |bencher| {
            bencher.iter(|| schedule.run(&mut world));
        });
    }

    group.finish();
}

pub fn wide_fan_out_run(criterion: &mut Criterion) {
    fn root() {}
    fn leaf() {}

    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct RootSet;
    #[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct LeafSet(usize);

    let mut group = criterion.benchmark_group("wide_fan_out_run");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for leaf_count in [16usize, 64, 256] {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.add_systems(root.in_set(RootSet));
        for index in 0..leaf_count {
            schedule.add_systems(leaf.in_set(LeafSet(index)).after(RootSet));
        }
        schedule.run(&mut world);

        group.bench_function(format!("{leaf_count}_dependents"), |bencher| {
            bencher.iter(|| schedule.run(&mut world));
        });
    }

    group.finish();
}

pub fn deferred_barrier_stress(criterion: &mut Criterion) {
    fn writer(mut commands: Commands) {
        commands.queue(|world: &mut World| {
            let mut value = world.resource_mut::<Counter>();
            value.0 = value.0.wrapping_add(1);
        });
    }

    fn reader(_counter: Option<Res<Counter>>) {}

    #[derive(Resource, Default)]
    struct Counter(u32);

    let mut group = criterion.benchmark_group("deferred_barrier_stress");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for pair_count in [8usize, 32, 128] {
        let mut world = World::new();
        world.init_resource::<Counter>();
        let mut schedule = Schedule::default();
        for _ in 0..pair_count {
            schedule.add_systems((writer, reader).chain());
        }
        schedule.run(&mut world);

        group.bench_function(format!("{pair_count}_pairs"), |bencher| {
            bencher.iter(|| schedule.run(&mut world));
        });
    }

    group.finish();
}

pub fn mixed_lane_run(criterion: &mut Criterion) {
    fn worker() {
        black_box(17usize);
    }

    fn non_send(_marker: NonSend<NonSendMarker>) {
        black_box(23usize);
    }

    fn exclusive(_world: &mut World) {
        black_box(31usize);
    }

    struct NonSendMarker(Rc<()>);

    let mut group = criterion.benchmark_group("mixed_lane_run");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for lane_groups in [4usize, 16, 64] {
        let mut world = World::new();
        world.insert_non_send(NonSendMarker(Rc::new(())));
        let mut schedule = Schedule::default();
        for _ in 0..lane_groups {
            schedule.add_systems((worker, non_send, exclusive));
        }
        schedule.run(&mut world);

        group.bench_function(format!("{lane_groups}_triples"), |bencher| {
            bencher.iter(|| schedule.run(&mut world));
        });
    }

    group.finish();
}
