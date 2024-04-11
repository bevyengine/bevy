#![allow(dead_code)]

use bevy_ecs::prelude::*;
use glam::*;

#[derive(Component, Copy, Clone)]
struct A(Mat4);
#[derive(Component, Copy, Clone)]
struct B(Mat4);
#[derive(Component, Copy, Clone)]
struct C(Mat4);
#[derive(Component, Copy, Clone)]
struct D(Mat4);
#[derive(Component, Copy, Clone)]
struct E(Mat4);
#[derive(Component, Copy, Clone)]
struct F(Mat4);
#[derive(Component, Copy, Clone)]
struct A2(Mat4);
#[derive(Component, Copy, Clone)]
struct B2(Mat4);
#[derive(Component, Copy, Clone)]
struct C2(Mat4);
#[derive(Component, Copy, Clone)]
struct D2(Mat4);
#[derive(Component, Copy, Clone)]
struct E2(Mat4);
#[derive(Component, Copy, Clone)]
struct F2(Mat4);
#[derive(Component, Copy, Clone)]
struct A3(Mat4);
#[derive(Component, Copy, Clone)]
struct B3(Mat4);
#[derive(Component, Copy, Clone)]
struct C3(Mat4);
#[derive(Component, Copy, Clone)]
struct D3(Mat4);
#[derive(Component, Copy, Clone)]
struct E3(Mat4);
#[derive(Component, Copy, Clone)]
struct F3(Mat4);

pub struct Benchmark(World, Vec<Entity>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::default();
        let mut entities = Vec::with_capacity(10_000);
        for _ in 0..10_000 {
            entities.push(
                world
                    .spawn((
                        A(Mat4::from_scale(Vec3::ONE)),
                        B(Mat4::from_scale(Vec3::ONE)),
                        C(Mat4::from_scale(Vec3::ONE)),
                        D(Mat4::from_scale(Vec3::ONE)),
                        E(Mat4::from_scale(Vec3::ONE)),
                        A2(Mat4::from_scale(Vec3::ONE)),
                        B2(Mat4::from_scale(Vec3::ONE)),
                        C2(Mat4::from_scale(Vec3::ONE)),
                        D2(Mat4::from_scale(Vec3::ONE)),
                        E2(Mat4::from_scale(Vec3::ONE)),
                        A3(Mat4::from_scale(Vec3::ONE)),
                        B3(Mat4::from_scale(Vec3::ONE)),
                        C3(Mat4::from_scale(Vec3::ONE)),
                        D3(Mat4::from_scale(Vec3::ONE)),
                        E3(Mat4::from_scale(Vec3::ONE)),
                    ))
                    .id(),
            );
        }

        Self(world, entities)
    }

    pub fn run(&mut self) {
        for entity in &self.1 {
            self.0
                .entity_mut(*entity)
                .insert(F(Mat4::from_scale(Vec3::ONE)));
        }

        for entity in &self.1 {
            self.0.entity_mut(*entity).remove::<F>();
        }
    }
}
