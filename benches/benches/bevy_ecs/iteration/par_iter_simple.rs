use bevy_ecs::prelude::*;
use bevy_tasks::{ComputeTaskPool, TaskPool};
use glam::*;

#[derive(Component, Copy, Clone)]
struct Transform(Mat4);

#[derive(Component, Copy, Clone)]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Rotation(Vec3);

#[derive(Component, Copy, Clone)]
struct Velocity(Vec3);

#[derive(Component, Copy, Clone, Default)]
struct Data<const X: u16>(f32);
pub struct Benchmark<'w>(World, QueryState<(&'w Velocity, &'w mut Position)>);

fn insert_if_bit_enabled<const B: u16>(entity: &mut EntityWorldMut, i: u16) {
    if i & 1 << B != 0 {
        entity.insert(Data::<B>(1.0));
    }
}

impl<'w> Benchmark<'w> {
    pub fn new(fragment: u16) -> Self {
        ComputeTaskPool::get_or_init(TaskPool::default);

        let mut world = World::new();

        let iter = world.spawn_batch(
            std::iter::repeat((
                Transform(Mat4::from_scale(Vec3::ONE)),
                Position(Vec3::X),
                Rotation(Vec3::X),
                Velocity(Vec3::X),
            ))
            .take(100_000),
        );
        let entities = iter.into_iter().collect::<Vec<Entity>>();
        for i in 0..fragment {
            let mut e = world.entity_mut(entities[i as usize]);
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

        let query = world.query::<(&Velocity, &mut Position)>();
        Self(world, query)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        self.1
            .par_iter_mut(&mut self.0)
            .for_each(|(v, mut p)| p.0 += v.0);
    }
}
