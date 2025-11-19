use core::hint::black_box;

use bevy_ecs::{
    component::Component,
    system::{Command, Commands},
    world::{CommandQueue, World},
};
use criterion::Criterion;

#[derive(Component)]
struct A;
#[derive(Component)]
struct B;
#[derive(Component)]
struct C;

pub fn empty_commands(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("empty_commands");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    group.bench_function("0_entities", |bencher| {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();

        bencher.iter(|| {
            command_queue.apply(&mut world);
        });
    });

    group.finish();
}

pub fn spawn_commands(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("spawn_commands");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in [100, 1_000, 10_000] {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            let mut world = World::default();
            let mut command_queue = CommandQueue::default();

            bencher.iter(|| {
                let mut commands = Commands::new(&mut command_queue, &world);
                for i in 0..entity_count {
                    let mut entity = commands.spawn_empty();
                    entity
                        .insert_if(A, || black_box(i % 2 == 0))
                        .insert_if(B, || black_box(i % 3 == 0))
                        .insert_if(C, || black_box(i % 4 == 0));

                    if black_box(i % 5 == 0) {
                        entity.despawn();
                    }
                }
                command_queue.apply(&mut world);
            });
        });
    }

    group.finish();
}

pub fn nonempty_spawn_commands(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("nonempty_spawn_commands");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for entity_count in [100, 1_000, 10_000] {
        group.bench_function(format!("{entity_count}_entities"), |bencher| {
            let mut world = World::default();
            let mut command_queue = CommandQueue::default();

            bencher.iter(|| {
                let mut commands = Commands::new(&mut command_queue, &world);
                for i in 0..entity_count {
                    if black_box(i % 2 == 0) {
                        commands.spawn(A);
                    }
                }
                command_queue.apply(&mut world);
            });
        });
    }

    group.finish();
}

#[derive(Default, Component)]
struct Matrix([[f32; 4]; 4]);

#[derive(Default, Component)]
struct Vec3([f32; 3]);

pub fn insert_commands(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("insert_commands");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    let entity_count = 10_000;
    group.bench_function("insert", |bencher| {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let mut entities = Vec::new();
        for _ in 0..entity_count {
            entities.push(world.spawn_empty().id());
        }

        bencher.iter(|| {
            let mut commands = Commands::new(&mut command_queue, &world);
            for entity in &entities {
                commands
                    .entity(*entity)
                    .insert((Matrix::default(), Vec3::default()));
            }
            command_queue.apply(&mut world);
        });
    });
    group.bench_function("insert_batch", |bencher| {
        let mut world = World::default();
        let mut command_queue = CommandQueue::default();
        let mut entities = Vec::new();
        for _ in 0..entity_count {
            entities.push(world.spawn_empty().id());
        }

        bencher.iter(|| {
            let mut commands = Commands::new(&mut command_queue, &world);
            let mut values = Vec::with_capacity(entity_count);
            for entity in &entities {
                values.push((*entity, (Matrix::default(), Vec3::default())));
            }
            commands.insert_batch(values);
            command_queue.apply(&mut world);
        });
    });

    group.finish();
}

struct FakeCommandA;
struct FakeCommandB(u64);

impl Command for FakeCommandA {
    fn apply(self, world: &mut World) {
        black_box(self);
        black_box(world);
    }
}

impl Command for FakeCommandB {
    fn apply(self, world: &mut World) {
        black_box(self);
        black_box(world);
    }
}

pub fn fake_commands(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("fake_commands");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for command_count in [100, 1_000, 10_000] {
        group.bench_function(format!("{command_count}_commands"), |bencher| {
            let mut world = World::default();
            let mut command_queue = CommandQueue::default();

            bencher.iter(|| {
                let mut commands = Commands::new(&mut command_queue, &world);
                for i in 0..command_count {
                    if black_box(i % 2 == 0) {
                        commands.queue(FakeCommandA);
                    } else {
                        commands.queue(FakeCommandB(0));
                    }
                }
                command_queue.apply(&mut world);
            });
        });
    }

    group.finish();
}

#[derive(Default)]
struct SizedCommand<T: Default + Send + Sync + 'static>(T);

impl<T: Default + Send + Sync + 'static> Command for SizedCommand<T> {
    fn apply(self, world: &mut World) {
        black_box(self);
        black_box(world);
    }
}

struct LargeStruct([u64; 64]);

impl Default for LargeStruct {
    fn default() -> Self {
        Self([0; 64])
    }
}

pub fn sized_commands_impl<T: Default + Command>(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group(format!("sized_commands_{}_bytes", size_of::<T>()));
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));

    for command_count in [100, 1_000, 10_000] {
        group.bench_function(format!("{command_count}_commands"), |bencher| {
            let mut world = World::default();
            let mut command_queue = CommandQueue::default();

            bencher.iter(|| {
                let mut commands = Commands::new(&mut command_queue, &world);
                for _ in 0..command_count {
                    commands.queue(T::default());
                }
                command_queue.apply(&mut world);
            });
        });
    }

    group.finish();
}

pub fn zero_sized_commands(criterion: &mut Criterion) {
    sized_commands_impl::<SizedCommand<()>>(criterion);
}

pub fn medium_sized_commands(criterion: &mut Criterion) {
    sized_commands_impl::<SizedCommand<(u32, u32, u32)>>(criterion);
}

pub fn large_sized_commands(criterion: &mut Criterion) {
    sized_commands_impl::<SizedCommand<LargeStruct>>(criterion);
}
