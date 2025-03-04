use bevy_ecs::prelude::*;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use rand::{prelude::SliceRandom, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[derive(Component, Copy, Clone)]
struct TableData(f32);

#[derive(Component, Copy, Clone)]
#[component(storage = "SparseSet")]
struct SparseData(f32);

fn deterministic_rand() -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(42)
}
pub struct Benchmark<'w>(World, QueryState<(&'w mut TableData, &'w SparseData)>);

impl<'w> Benchmark<'w> {
    pub fn new() -> Self {
        let mut world = World::new();
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut v = vec![];
        for _ in 0..100000 {
            world.spawn((TableData(0.0), SparseData(0.0)));
            v.push(world.spawn(TableData(0.)).id());
        }

        // by shuffling ,randomize the archetype iteration order to significantly deviate from the table order. This maximizes the loss of cache locality during archetype-based iteration.
        v.shuffle(&mut deterministic_rand());
        for e in v.into_iter() {
            world.entity_mut(e).despawn();
        }

        let query = world.query::<(&mut TableData, &SparseData)>();
        Self(world, query)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        self.1
            .par_iter_mut(&mut self.0)
            .for_each(|(mut v1, v2)| v1.0 += v2.0);
    }
}
