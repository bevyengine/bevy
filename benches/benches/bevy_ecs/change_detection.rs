use core::hint::black_box;

use bevy_ecs::{
    component::{Component, Mutable},
    entity::Entity,
    lifecycle::Insert,
    observer::{Observer, On},
    prelude::{Added, Changed, EntityWorldMut, QueryState},
    query::QueryFilter,
    world::World,
};
use criterion::{criterion_group, Criterion};
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

criterion_group!(
    benches,
    all_added_detection,
    all_changed_detection,
    few_changed_detection,
    none_changed_detection,
    multiple_archetype_none_changed_detection,
    all_immutable_replace_detection,
    few_immutable_replace_detection,
    none_immutable_replace_detection,
    all_immutable_replace_no_observer,
    few_immutable_replace_no_observer
);

macro_rules! modify {
    ($components:ident;$($index:tt),*) => {
        $(
            $components.$index.map(|mut v| {
                v.0+=1.
            });
        )*
    };
}
#[derive(Component, Default)]
#[component(storage = "Table")]
struct Table(f32);
#[derive(Component, Default)]
#[component(storage = "SparseSet")]
struct Sparse(f32);
#[derive(Component, Default)]
#[component(storage = "Table")]
struct Data<const X: u16>(f32);

#[derive(Component, Default, Clone)]
#[component(storage = "Table", immutable)]
struct ImmutableTable(f32);

#[derive(Component, Default, Clone)]
#[component(storage = "SparseSet", immutable)]
struct ImmutableSparse(f32);

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

trait BenchReplace {
    fn bench_replace(&self) -> Self;
}

impl BenchReplace for ImmutableTable {
    fn bench_replace(&self) -> Self {
        Self(self.0 + 1f32)
    }
}

impl BenchReplace for ImmutableSparse {
    fn bench_replace(&self) -> Self {
        Self(self.0 + 1f32)
    }
}

const ENTITIES_TO_BENCH_COUNT: &[u32] = &[5000, 50000];

type BenchGroup<'a> = criterion::BenchmarkGroup<'a, criterion::measurement::WallTime>;

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}

fn setup<T: Component + Default>(entity_count: u32) -> World {
    let mut world = World::default();
    world.spawn_batch((0..entity_count).map(|_| T::default()));
    black_box(world)
}

fn setup_with_observer<T: Component + Default + Clone + 'static>(entity_count: u32) -> World {
    let mut world = World::default();

    world.spawn(Observer::new(|trigger: On<Insert, T>| {
        black_box(trigger);
    }));

    world.spawn_batch((0..entity_count).map(|_| T::default()));
    black_box(world)
}

// create a cached query in setup to avoid extra costs in each iter
fn generic_filter_query<F: QueryFilter>(world: &mut World) -> QueryState<Entity, F> {
    world.query_filtered::<Entity, F>()
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
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = setup::<T>(entity_count);
                    let query = generic_filter_query::<Added<T>>(&mut world);
                    (world, query)
                },
                |(world, query)| {
                    let mut count = 0;
                    for entity in query.iter(world) {
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
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
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

fn all_changed_detection_generic<T: Component<Mutability = Mutable> + Default + BenchModify>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = setup::<T>(entity_count);
                    world.clear_trackers();
                    let mut query = world.query::<&mut T>();
                    for mut component in query.iter_mut(&mut world) {
                        black_box(component.bench_modify());
                    }
                    let query = generic_filter_query::<Changed<T>>(&mut world);
                    (world, query)
                },
                |(world, query)| {
                    let mut count = 0;
                    for entity in query.iter(world) {
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
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
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

fn few_changed_detection_generic<T: Component<Mutability = Mutable> + Default + BenchModify>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    let ratio_to_modify = 0.1;
    let amount_to_modify = (entity_count as f32 * ratio_to_modify) as usize;
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
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
                    let query = generic_filter_query::<Changed<T>>(&mut world);
                    (world, query)
                },
                |(world, query)| {
                    for entity in query.iter(world) {
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
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
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

fn none_changed_detection_generic<T: Component<Mutability = Mutable> + Default>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = setup::<T>(entity_count);
                    world.clear_trackers();
                    let query = generic_filter_query::<Changed<T>>(&mut world);
                    (world, query)
                },
                |(world, query)| {
                    let mut count = 0;
                    for entity in query.iter(world) {
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
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
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

fn all_immutable_replace_detection_generic<
    T: Component + Default + Clone + BenchReplace + 'static,
>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = setup_with_observer::<T>(entity_count);
                    let mut query = world.query::<Entity>();
                    let entities: Vec<_> = query.iter(&world).collect();
                    (world, entities)
                },
                |(world, entities)| {
                    for entity in entities {
                        let component = world
                            .entity(*entity)
                            .get::<T>()
                            .cloned()
                            .unwrap_or_else(T::default);
                        let new_component = component.bench_replace();
                        black_box(world.entity_mut(*entity).insert(new_component));
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn all_immutable_replace_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("all_immutable_replace_detection");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
        generic_bench(
            &mut group,
            vec![
                Box::new(all_immutable_replace_detection_generic::<ImmutableTable>),
                //Box::new(all_immutable_replace_detection_generic::<ImmutableSparse>),
            ],
            entity_count,
        );
    }
}

fn few_immutable_replace_detection_generic<
    T: Component + Default + Clone + BenchReplace + 'static,
>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    let ratio_to_modify = 0.1;
    let amount_to_modify = (entity_count as f32 * ratio_to_modify) as usize;
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = setup_with_observer::<T>(entity_count);
                    let mut query = world.query::<Entity>();
                    let mut entities: Vec<_> = query.iter(&world).collect();
                    entities.shuffle(&mut deterministic_rand());
                    (world, entities)
                },
                |(world, entities)| {
                    for entity in entities[0..amount_to_modify].iter() {
                        let component = world
                            .entity(*entity)
                            .get::<T>()
                            .cloned()
                            .unwrap_or_else(T::default);
                        let new_component = component.bench_replace();
                        black_box(world.entity_mut(*entity).insert(new_component));
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn few_immutable_replace_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("few_immutable_replace_detection");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
        generic_bench(
            &mut group,
            vec![
                Box::new(few_immutable_replace_detection_generic::<ImmutableTable>),
                Box::new(few_immutable_replace_detection_generic::<ImmutableSparse>),
            ],
            entity_count,
        );
    }
}

fn none_immutable_replace_detection_generic<T: Component + Default + Clone + 'static>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let world = setup_with_observer::<T>(entity_count);
                    world
                },
                |world| {
                    // Do nothing - just measure the overhead of having the observer
                    // without actually replacing any components
                    world.clear_trackers();
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn none_immutable_replace_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("none_immutable_replace_detection");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
        generic_bench(
            &mut group,
            vec![
                Box::new(none_immutable_replace_detection_generic::<ImmutableTable>),
                Box::new(none_immutable_replace_detection_generic::<ImmutableSparse>),
            ],
            entity_count,
        );
    }
}

// Baseline benchmarks without observers for comparison
fn all_immutable_replace_no_observer_generic<
    T: Component + Default + Clone + BenchReplace + 'static,
>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = setup::<T>(entity_count); // No observer
                    let mut query = world.query::<Entity>();
                    let entities: Vec<_> = query.iter(&world).collect();
                    (world, entities)
                },
                |(world, entities)| {
                    for entity in entities {
                        let component = world
                            .entity(*entity)
                            .get::<T>()
                            .cloned()
                            .unwrap_or_else(T::default);
                        let new_component = component.bench_replace();
                        black_box(world.entity_mut(*entity).insert(new_component));
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn all_immutable_replace_no_observer(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("all_immutable_replace_no_observer");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
        generic_bench(
            &mut group,
            vec![
                Box::new(all_immutable_replace_no_observer_generic::<ImmutableTable>),
                Box::new(all_immutable_replace_no_observer_generic::<ImmutableSparse>),
            ],
            entity_count,
        );
    }
}

fn few_immutable_replace_no_observer_generic<
    T: Component + Default + Clone + BenchReplace + 'static,
>(
    group: &mut BenchGroup,
    entity_count: u32,
) {
    let ratio_to_modify = 0.1;
    let amount_to_modify = (entity_count as f32 * ratio_to_modify) as usize;
    group.bench_function(
        format!("{}_entities_{}", entity_count, core::any::type_name::<T>()),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = setup::<T>(entity_count); // No observer
                    let mut query = world.query::<Entity>();
                    let mut entities: Vec<_> = query.iter(&world).collect();
                    entities.shuffle(&mut deterministic_rand());
                    (world, entities)
                },
                |(world, entities)| {
                    for entity in entities[0..amount_to_modify].iter() {
                        let component = world
                            .entity(*entity)
                            .get::<T>()
                            .cloned()
                            .unwrap_or_else(T::default);
                        let new_component = component.bench_replace();
                        black_box(world.entity_mut(*entity).insert(new_component));
                    }
                },
                criterion::BatchSize::LargeInput,
            );
        },
    );
}

fn few_immutable_replace_no_observer(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("few_immutable_replace_no_observer");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(4));
    for &entity_count in ENTITIES_TO_BENCH_COUNT {
        generic_bench(
            &mut group,
            vec![
                Box::new(few_immutable_replace_no_observer_generic::<ImmutableTable>),
                Box::new(few_immutable_replace_no_observer_generic::<ImmutableSparse>),
            ],
            entity_count,
        );
    }
}

fn insert_if_bit_enabled<const B: u16>(entity: &mut EntityWorldMut, i: u16) {
    if i & (1 << B) != 0 {
        entity.insert(Data::<B>(1.0));
    }
}

fn add_archetypes_entities<T: Component<Mutability = Mutable> + Default>(
    world: &mut World,
    archetype_count: u16,
    entity_count: u32,
) {
    for i in 0..archetype_count {
        for _j in 0..entity_count {
            let mut e = world.spawn(T::default());
            insert_if_bit_enabled::<0>(&mut e, i);
            insert_if_bit_enabled::<1>(&mut e, i);
            insert_if_bit_enabled::<2>(&mut e, i);
            insert_if_bit_enabled::<3>(&mut e, i);
            insert_if_bit_enabled::<4>(&mut e, i);
            insert_if_bit_enabled::<5>(&mut e, i);
            insert_if_bit_enabled::<6>(&mut e, i);
            insert_if_bit_enabled::<7>(&mut e, i);
            insert_if_bit_enabled::<8>(&mut e, i);
            insert_if_bit_enabled::<9>(&mut e, i);
            insert_if_bit_enabled::<10>(&mut e, i);
            insert_if_bit_enabled::<11>(&mut e, i);
            insert_if_bit_enabled::<12>(&mut e, i);
            insert_if_bit_enabled::<13>(&mut e, i);
            insert_if_bit_enabled::<14>(&mut e, i);
            insert_if_bit_enabled::<15>(&mut e, i);
        }
    }
}
fn multiple_archetype_none_changed_detection_generic<
    T: Component<Mutability = Mutable> + Default + BenchModify,
>(
    group: &mut BenchGroup,
    archetype_count: u16,
    entity_count: u32,
) {
    group.bench_function(
        format!(
            "{}_archetypes_{}_entities_{}",
            archetype_count,
            entity_count,
            core::any::type_name::<T>()
        ),
        |bencher| {
            bencher.iter_batched_ref(
                || {
                    let mut world = World::new();
                    add_archetypes_entities::<T>(&mut world, archetype_count, entity_count);
                    world.clear_trackers();
                    let mut query = world.query::<(
                        Option<&mut Data<0>>,
                        Option<&mut Data<1>>,
                        Option<&mut Data<2>>,
                        Option<&mut Data<3>>,
                        Option<&mut Data<4>>,
                        Option<&mut Data<5>>,
                        Option<&mut Data<6>>,
                        Option<&mut Data<7>>,
                        Option<&mut Data<8>>,
                        Option<&mut Data<9>>,
                        Option<&mut Data<10>>,
                        Option<&mut Data<11>>,
                        Option<&mut Data<12>>,
                        Option<&mut Data<13>>,
                        Option<&mut Data<14>>,
                    )>();
                    for components in query.iter_mut(&mut world) {
                        // change Data<X> while keeping T unchanged
                        modify!(components;0,1,2,3,4,5,6,7,8,9,10,11,12,13,14);
                    }
                    let query = generic_filter_query::<Changed<T>>(&mut world);
                    (world, query)
                },
                |(world, query)| {
                    let mut count = 0;
                    for entity in query.iter(world) {
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

fn multiple_archetype_none_changed_detection(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("multiple_archetypes_none_changed_detection");
    group.warm_up_time(core::time::Duration::from_millis(800));
    group.measurement_time(core::time::Duration::from_secs(8));
    for archetype_count in [5, 20, 100] {
        for entity_count in [10, 100, 1000, 10000] {
            multiple_archetype_none_changed_detection_generic::<Table>(
                &mut group,
                archetype_count,
                entity_count,
            );
            multiple_archetype_none_changed_detection_generic::<Sparse>(
                &mut group,
                archetype_count,
                entity_count,
            );
        }
    }
}
