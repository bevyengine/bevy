use bevy_ecs::prelude::*;
use glam::*;

#[derive(Component, Copy, Clone)]
struct Transform(Mat4);

#[derive(Component, Copy, Clone)]
#[component(storage = "SparseSet")]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Rotation(Vec3);

#[derive(Component, Copy, Clone)]
#[component(storage = "SparseSet")]
struct Velocity(Vec3);

#[derive(Component, Copy, Clone, Eq, PartialEq, Hash)]
#[component(storage = "Shared")]
struct VelocityModifier(u32);

pub struct Benchmark<'w>(
    World,
    QueryState<(&'w Velocity, &'w mut Position, &'w VelocityModifier)>,
);

impl<'w> Benchmark<'w> {
    pub fn new() -> Self {
        let mut world = World::new();

        world.spawn_batch((0..10_000).map(|i| {
            (
                Transform(Mat4::from_scale(Vec3::ONE)),
                Position(Vec3::X),
                Rotation(Vec3::X),
                Velocity(Vec3::X),
                VelocityModifier(i / 100),
            )
        }));

        let query = world.query::<(&Velocity, &mut Position, &VelocityModifier)>();
        Self(world, query)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        for (velocity, mut position, modifier) in self.1.iter_mut(&mut self.0) {
            position.0 += velocity.0 * (modifier.0 as f32);
        }
    }
}
