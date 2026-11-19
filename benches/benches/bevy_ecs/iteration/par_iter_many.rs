use bevy_ecs::{
    entity::{EntitySetIterator, UniqueEntityVec},
    prelude::*,
};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use core::hint::black_box;
use glam::*;
use std::collections::BTreeSet;

#[derive(Component, Copy, Clone)]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Velocity(Vec3);

#[derive(Component, Copy, Clone, Default)]
struct Data<const X: u16>(f32);

pub struct Benchmark<'w> {
    world: World,
    query: QueryState<(&'w Velocity, &'w mut Position)>,
    entities: Vec<Entity>,
    unique_entities: UniqueEntityVec,
}

fn insert_if_bit_enabled<const B: u16>(entity: &mut EntityWorldMut, i: u16) {
    if i & (1 << B) != 0 {
        entity.insert(Data::<B>(1.0));
    }
}

impl<'w> Benchmark<'w> {
    /// Spawns 100,000 entities, splits the first `fragment` of them into
    /// different archetypes, and queries every other entity by id.
    pub fn new(fragment: u16) -> Self {
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut world = World::new();

        let all = world
            .spawn_batch(core::iter::repeat_n(
                (Position(Vec3::X), Velocity(Vec3::X)),
                100_000,
            ))
            .collect::<Vec<Entity>>();
        for i in 0..fragment {
            let mut e = world.entity_mut(all[i as usize]);
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
        }

        let entities = all.into_iter().step_by(2).collect::<Vec<Entity>>();
        let unique_entities = entities
            .iter()
            .copied()
            .collect::<BTreeSet<Entity>>()
            .into_iter()
            .collect_set::<UniqueEntityVec>();

        let query = world.query::<(&Velocity, &mut Position)>();
        Self {
            world,
            query,
            entities,
            unique_entities,
        }
    }

    #[inline(never)]
    pub fn run_many(&mut self) {
        self.query
            .query(&self.world)
            .par_iter_many(&self.entities)
            .for_each(|item| {
                if let Ok((v, p)) = item {
                    black_box(v.0 + p.0);
                }
            });
    }

    #[inline(never)]
    pub fn run_many_unique(&mut self) {
        self.query
            .query(&self.world)
            .par_iter_many_unique(&self.unique_entities)
            .for_each(|item| {
                if let Ok((v, p)) = item {
                    black_box(v.0 + p.0);
                }
            });
    }

    #[inline(never)]
    pub fn run_many_unique_mut(&mut self) {
        self.query
            .query_mut(&mut self.world)
            .par_iter_many_unique_mut(&self.unique_entities)
            .for_each(|item| {
                if let Ok((v, mut p)) = item {
                    p.0 += v.0;
                }
            });
    }
}
