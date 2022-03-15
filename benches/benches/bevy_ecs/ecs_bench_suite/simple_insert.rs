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

pub struct Benchmark;

impl Benchmark {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self) {
        let mut world = World::new();
        world.spawn_batch((0..10_000).map(|_| {
            (
                Transform(Matrix4::from_scale(1.0)),
                Position(Vector3::unit_x()),
                Rotation(Vector3::unit_x()),
                Velocity(Vector3::unit_x()),
            )
        }));
        // world.spawn_batch_new(SoaBatch::new((
        //     vec![Transform(Matrix4::from_scale(1.0)); 10000],
        //     vec![Position(Vector3::unit_x()); 10000],
        //     vec![Rotation(Vector3::unit_x()); 10000],
        //     vec![Velocity(Vector3::unit_x()); 10000],
        // )));
    }
}
