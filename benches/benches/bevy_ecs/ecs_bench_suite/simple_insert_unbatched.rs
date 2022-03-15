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
        for _ in 0..10_000 {
            world.spawn().insert_bundle((
                Transform(Matrix4::from_scale(1.0)),
                Position(Vector3::unit_x()),
                Rotation(Vector3::unit_x()),
                Velocity(Vector3::unit_x()),
            ));
        }
    }
}
