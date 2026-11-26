//! Per-entity against contiguous parallel iteration.

use bevy_ecs::batching::BatchingStrategy;
use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use glam::*;

const ENTITIES: usize = 100_000;

#[derive(Component, Copy, Clone)]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Health(f32);

#[derive(Component, Copy, Clone)]
struct Pace(f32);

#[derive(Component, Copy, Clone)]
struct Enemy {
    speed: f32,
    sway: f32,
    mend: f32,
    max_health: f32,
}

#[derive(Component, Copy, Clone)]
struct Seed(f32);

#[derive(Clone, Copy)]
pub enum Mode {
    PerEntity,
    Contiguous,
    /// Contiguous slices accessed by index, exercising per-row change tracking.
    ContiguousIndexed,
}

/// Named query data for the benchmark, including contiguous support.
#[derive(QueryData)]
#[query_data(mutable, contiguous(mutable))]
struct EnemyQuery {
    enemy: &'static Enemy,
    seed: &'static Seed,
    pace: &'static Pace,
    position: &'static mut Position,
    health: &'static mut Health,
}

pub struct Benchmark {
    world: World,
    query: QueryState<EnemyQuery>,
    mode: Mode,
    strategy: Option<BatchingStrategy>,
}

#[inline(always)]
fn step(position: &mut Vec3, health: &mut f32, enemy: Enemy, seed: f32, pace: f32) {
    let inward = -position.normalize_or_zero();
    let phase = seed + position.x * 0.001;
    let sway = Vec3::new(-inward.y, inward.x, 0.0) * phase.sin() * enemy.sway;
    *position += (inward + sway) * enemy.speed * pace;
    if *health < enemy.max_health {
        *health = (*health + enemy.mend).min(enemy.max_health);
    }
}

fn enemy_bundle(i: usize) -> (Position, Health, Pace, Enemy, Seed) {
    (
        Position(Vec3::splat(i as f32 * 0.001)),
        Health(50.0),
        Pace(1.0),
        Enemy {
            speed: 0.05,
            sway: 0.35,
            mend: 0.1,
            max_health: 100.0,
        },
        Seed(i as f32 * 0.618),
    )
}

// Six zero-sized markers make 64 archetypes with roughly equal row counts.
#[derive(Component)]
struct MarkA;
#[derive(Component)]
struct MarkB;
#[derive(Component)]
struct MarkC;
#[derive(Component)]
struct MarkD;
#[derive(Component)]
struct MarkE;
#[derive(Component)]
struct MarkF;

fn insert_marks(entity: &mut EntityWorldMut, bits: usize) {
    if bits & 1 != 0 {
        entity.insert(MarkA);
    }
    if bits & 2 != 0 {
        entity.insert(MarkB);
    }
    if bits & 4 != 0 {
        entity.insert(MarkC);
    }
    if bits & 8 != 0 {
        entity.insert(MarkD);
    }
    if bits & 16 != 0 {
        entity.insert(MarkE);
    }
    if bits & 32 != 0 {
        entity.insert(MarkF);
    }
}

fn uniform_world() -> World {
    let mut world = World::new();
    world.spawn_batch((0..ENTITIES).map(enemy_bundle));
    world
}

fn fragmented_world() -> World {
    let mut world = World::new();
    let entities: Vec<Entity> = world.spawn_batch((0..ENTITIES).map(enemy_bundle)).collect();
    for (i, &entity) in entities.iter().enumerate() {
        insert_marks(&mut world.entity_mut(entity), i % 64);
    }
    world
}

impl Benchmark {
    /// Benchmarks a uniform world with `batches` per thread.
    pub fn new(mode: Mode, batches: usize) -> Self {
        Self::from_world(
            mode,
            uniform_world(),
            Some(BatchingStrategy::new().batches_per_thread(batches)),
        )
    }

    /// Benchmarks a world fragmented across 64 archetypes with default batching.
    pub fn fragmented(mode: Mode) -> Self {
        Self::from_world(mode, fragmented_world(), None)
    }

    fn from_world(mode: Mode, mut world: World, strategy: Option<BatchingStrategy>) -> Self {
        ComputeTaskPool::get_or_init(TaskPool::default);
        let query = world.query::<EnemyQuery>();
        Self {
            world,
            query,
            mode,
            strategy,
        }
    }

    #[inline(never)]
    pub fn run(&mut self) {
        let strategy = self.strategy.clone();
        match self.mode {
            Mode::PerEntity => {
                let mut iter = self.query.par_iter_mut(&mut self.world);
                if let Some(strategy) = strategy {
                    iter = iter.batching_strategy(strategy);
                }
                iter.for_each(|mut item| {
                    step(
                        &mut item.position.0,
                        &mut item.health.0,
                        *item.enemy,
                        item.seed.0,
                        item.pace.0,
                    );
                });
            }
            Mode::Contiguous => {
                let mut iter = self.query.contiguous_par_iter_mut(&mut self.world).unwrap();
                if let Some(strategy) = strategy {
                    iter = iter.batching_strategy(strategy);
                }
                iter.for_each(|mut item| {
                    // Deref once per batch, then index the plain slices.
                    let positions = &mut *item.position;
                    let healths = &mut *item.health;
                    for i in 0..positions.len() {
                        step(
                            &mut positions[i].0,
                            &mut healths[i].0,
                            item.enemy[i],
                            item.seed[i].0,
                            item.pace[i].0,
                        );
                    }
                });
            }
            Mode::ContiguousIndexed => {
                let mut iter = self.query.contiguous_par_iter_mut(&mut self.world).unwrap();
                if let Some(strategy) = strategy {
                    iter = iter.batching_strategy(strategy);
                }
                iter.for_each(|mut item| {
                    for i in 0..item.position.len() {
                        step(
                            &mut item.position[i].0,
                            &mut item.health[i].0,
                            item.enemy[i],
                            item.seed[i].0,
                            item.pace[i].0,
                        );
                    }
                });
            }
        }
    }
}
