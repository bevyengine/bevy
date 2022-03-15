use bevy_ecs::prelude::*;
use cgmath::*;

#[derive(Component, Copy, Clone)]
struct Transform(Matrix4<f32>);

#[derive(Component, Copy, Clone)]
struct Position(Vector3<f32>);

#[derive(Component, Copy, Clone)]
struct Rotation(Vector3<f32>);

#[derive(Component, Copy, Clone)]
struct Velocity(Vector3<f32>);

pub struct Benchmark<'w>(World, QueryState<(&'w Velocity, &'w mut Position)>);

impl<'w> Benchmark<'w> {
    pub fn new() -> Self {
        let mut world = World::new();

        // TODO: batch this
        for _ in 0..10_000 {
            world.spawn().insert_bundle((
                Transform(Matrix4::from_scale(1.0)),
                Position(Vector3::unit_x()),
                Rotation(Vector3::unit_x()),
                Velocity(Vector3::unit_x()),
            ));
        }

        let query = world.query::<(&Velocity, &mut Position)>();
        Self(world, query)
    }

    pub fn run(&mut self) {
        self.1.for_each_mut(&mut self.0, |(velocity, mut position)| {
            position.0 += velocity.0;
        });
    }
}
