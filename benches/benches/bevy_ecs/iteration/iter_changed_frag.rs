use bevy_ecs::{change_detection::Tick, prelude::*};
use glam::*;

#[derive(Component, Copy, Clone)]
struct Transform(Mat4);

#[derive(Component, Copy, Clone)]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Rotation(Vec3);

#[derive(Component, Copy, Clone)]
#[component(summary_tick)]
struct Velocity(Vec3);

#[derive(Component, Copy, Clone, Default)]
struct Data<const X: u16>(f32);

fn insert_if_bit_enabled<const B: u16>(entity: &mut EntityWorldMut, i: u16) {
    if i & (1 << B) != 0 {
        entity.insert(Data::<B>(1.0));
    }
}

pub struct Benchmark<'w> {
    world: World,
    query: QueryState<(&'w Velocity, &'w mut Position), Changed<Velocity>>,
    change_tick_after_spawn: Tick,
}

impl<'w> Benchmark<'w> {
    pub fn new(fragment: u16) -> Self {
        let mut world = World::new();

        let entities = world
            .spawn_batch(core::iter::repeat_n(
                (
                    Transform(Mat4::from_scale(Vec3::ONE)),
                    Position(Vec3::X),
                    Rotation(Vec3::X),
                    Velocity(Vec3::X),
                ),
                10_000,
            ))
            .collect::<Vec<_>>();

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

        world.increment_change_tick();
        let change_tick_after_spawn = world.change_tick();
        world.increment_change_tick();

        world
            .entity_mut(entities[1])
            .get_mut::<Velocity>()
            .unwrap()
            .set_changed();

        world.increment_change_tick();

        let query = world.query_filtered::<(&Velocity, &mut Position), Changed<Velocity>>();
        Self {
            world,
            query,
            change_tick_after_spawn,
        }
    }

    #[inline(never)]
    pub fn run(&mut self) {
        self.world
            .last_change_tick_scope(self.change_tick_after_spawn, |world| {
                for (velocity, mut position) in self.query.iter_mut(world) {
                    position.0 += velocity.0;
                }
            });
    }
}
