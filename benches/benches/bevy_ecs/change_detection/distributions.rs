use std::hint::black_box;

use bevy_ecs::{
    component::{Component, Mutable},
    entity::Entity,
    query::{Changed, QueryState},
    world::World,
};
use criterion::{criterion_group, BatchSize, Criterion};
use rand::{rngs::StdRng, seq::SliceRandom, SeedableRng};

criterion_group!(benches, distributions);

#[derive(Clone, Copy, Component, Default)]
#[component(change = "indexed")]
struct IndexedComponent(u32);

#[derive(Clone, Copy, Component, Default)]
struct NonIndexedComponent(u32);

trait TestComponent: Component<Mutability = Mutable> + Default {
    fn mutate(&mut self);
}

impl TestComponent for IndexedComponent {
    fn mutate(&mut self) {
        self.0 += 1;
    }
}

impl TestComponent for NonIndexedComponent {
    fn mutate(&mut self) {
        self.0 += 1;
    }
}

fn distributions(criterion: &mut Criterion) {
    const ENTITY_COUNT: usize = 100000;
    let mut group = criterion.benchmark_group("distributions");

    for changed_entity_count in [0, 1, 10, 100, 1000, 10000, 100000] {
        group.bench_function(
            format!("indexed_distribution_changed_{changed_entity_count}"),
            |bencher| {
                bencher.iter_batched_ref(
                    || setup_benchmark::<IndexedComponent>(changed_entity_count),
                    |&mut (ref mut world, ref mut query)| {
                        run_benchmark::<IndexedComponent>(world, query, changed_entity_count);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
        group.bench_function(
            format!("non_indexed_distribution_changed_{changed_entity_count}"),
            |bencher| {
                bencher.iter_batched_ref(
                    || setup_benchmark::<NonIndexedComponent>(changed_entity_count),
                    |&mut (ref mut world, ref mut query)| {
                        run_benchmark::<NonIndexedComponent>(world, query, changed_entity_count);
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    fn setup_benchmark<C>(changed_entity_count: usize) -> (World, QueryState<Entity, Changed<C>>)
    where
        C: TestComponent,
    {
        let mut world = World::new();
        let mut order = vec![];
        for _ in 0..ENTITY_COUNT {
            order.push(world.spawn(C::default()).id());
        }
        order.shuffle(&mut StdRng::seed_from_u64(12345));
        world.clear_trackers();
        for entity in &order[0..changed_entity_count] {
            world.get_mut::<C>(*entity).unwrap().mutate();
        }
        let query = world.query_filtered::<Entity, Changed<C>>();
        (world, query)
    }

    fn run_benchmark<C>(
        world: &mut World,
        query: &mut QueryState<Entity, Changed<C>>,
        changed_entity_count: usize,
    ) where
        C: TestComponent,
    {
        let mut count = 0;
        for entity in query.iter(world) {
            black_box(entity);
            count += 1;
        }
        assert_eq!(count, changed_entity_count);
    }
}
