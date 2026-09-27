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

pub struct Benchmark<'w> {
    world: World,
    query: QueryState<(&'w Velocity, &'w mut Position), Changed<Velocity>>,
    change_tick_after_spawn: Tick,
}

impl<'w> Benchmark<'w> {
    pub fn new(with_changed: bool) -> Self {
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

        world.increment_change_tick();
        let change_tick_after_spawn = world.change_tick();
        world.increment_change_tick();

        if with_changed {
            world
                .entity_mut(entities[0])
                .get_mut::<Velocity>()
                .unwrap()
                .set_changed();
        }

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
