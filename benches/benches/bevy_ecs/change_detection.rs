use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::{Added, Changed},
    world::World,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

criterion_group!(
    benches,
    all_added_detection,
    all_changed_detection,
    few_changed_detection,
    none_changed_detection,
);
criterion_main!(benches);

#[derive(Component, Default)]
#[component(storage = "Table")]
struct Table(f32);
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
struct Sparse(f32);

trait BenchModify {
    fn bench_modify(&mut self) -> f32;
}

impl BenchModify for Table {
    fn bench_modify(&mut self) -> f32 {
        self.0 += 1f32;
        black_box(self.0)
    }
}
impl BenchModify for Sparse {
    fn bench_modify(&mut self) -> f32 {
        self.0 += 1f32;
        black_box(self.0)
    }
}

const RANGE_ENTITIES_TO_BENCH_COUNT: std::ops::Range<u32> = 5..7;

type BenchGroup<'a> = criterion::BenchmarkGroup<'a, criterion::measurement::WallTime>;

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

fn setup<T: Component + Default>(entity_count: u32) -> World {
    let mut world = World::default();
    world.spawn_batch((0..entity_count).map(|_| T::default()));
    black_box(world)
}

fn generic_bench<P: Copy>(
    bench_group: &mut BenchGroup,
    mut benches: Vec<Box<dyn FnMut(&mut BenchGroup, P)>>,
    bench_parameters: P,
) {
    for b in &mut benches {
        b(bench_group, bench_parameters);
    }
}

fn all_added_detection_generic<T: Component + Default>(group: &mut BenchGroup, entity_count: u32) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, std::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched(
                || setup::<T>(entity_count),
                |mut world| {
                    let mut count = 0;
                    let mut query = world.query_filtered::<Entity, Added<T>>();
                    for entity in query.iter(&world) {
                        black_box(entity);
                        count += 1;
                    }
                    assert_eq!(entity_count, count);
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn all_added_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("all_added_detection");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for entity_count in RANGE_ENTITIES_TO_BENCH_COUNT.map(|i| i * 10_000) {
        generic_bench(
            &mut group,
            vec![
                Box::new(all_added_detection_generic::<Table>),
                Box::new(all_added_detection_generic::<Sparse>),
            ],
            entity_count,
        );
    }
}

fn all_changed_detection_generic<T: Component + Default + BenchModify>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, std::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched(
                || {
                    let mut world = setup::<T>(entity_count);
                    world.clear_trackers();
                    let mut query = world.query::<&mut T>();
                    for mut component in query.iter_mut(&mut world) {
                        black_box(component.bench_modify());
                    }
                    world
                },
                |mut world| {
                    let mut count = 0;
                    let mut query = world.query_filtered::<Entity, Changed<T>>();
                    for entity in query.iter(&world) {
                        black_box(entity);
                        count += 1;
                    }
                    assert_eq!(entity_count, count);
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn all_changed_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("all_changed_detection");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for entity_count in RANGE_ENTITIES_TO_BENCH_COUNT.map(|i| i * 10_000) {
        generic_bench(
            &mut group,
            vec![
                Box::new(all_changed_detection_generic::<Table>),
                Box::new(all_changed_detection_generic::<Sparse>),
            ],
            entity_count,
        );
    }
}

fn few_changed_detection_generic<T: Component + Default + BenchModify>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    let ratio_to_modify = 0.1;
    let amount_to_modify = (entity_count as f32 * ratio_to_modify) as usize;
    group.bench_function(
        format!("{}_entities_{}", entity_count, std::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched(
                || {
                    let mut world = setup::<T>(entity_count);
                    world.clear_trackers();
                    let mut query = world.query::<&mut T>();
                    let mut to_modify: Vec<bevy_ecs::prelude::Mut<T>> =
                        query.iter_mut(&mut world).collect();
                    to_modify.shuffle(&mut deterministic_rand());
                    for component in to_modify[0..amount_to_modify].iter_mut() {
                        black_box(component.bench_modify());
                    }
                    world
                },
                |mut world| {
                    let mut query = world.query_filtered::<Entity, Changed<T>>();
                    for entity in query.iter(&world) {
                        black_box(entity);
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn few_changed_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("few_changed_detection");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for entity_count in RANGE_ENTITIES_TO_BENCH_COUNT.map(|i| i * 10_000) {
        generic_bench(
            &mut group,
            vec![
                Box::new(few_changed_detection_generic::<Table>),
                Box::new(few_changed_detection_generic::<Sparse>),
            ],
            entity_count,
        );
    }
}

fn none_changed_detection_generic<T: Component + Default>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, std::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched(
                || {
                    let mut world = setup::<T>(entity_count);
                    world.clear_trackers();
                    world
                },
                |mut world| {
                    let mut count = 0;
                    let mut query = world.query_filtered::<Entity, Changed<T>>();
                    for entity in query.iter(&world) {
                        black_box(entity);
                        count += 1;
                    }
                    assert_eq!(0, count);
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn none_changed_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("none_changed_detection");
    group.warm_up_time(std::time::Duration::from_millis(500));
    group.measurement_time(std::time::Duration::from_secs(4));
    for entity_count in RANGE_ENTITIES_TO_BENCH_COUNT.map(|i| i * 10_000) {
        generic_bench(
            &mut group,
            vec![
                Box::new(none_changed_detection_generic::<Table>),
                Box::new(none_changed_detection_generic::<Sparse>),
            ],
            entity_count,
        );
    }
}
