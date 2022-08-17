use std::marker::PhantomData;

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

#[derive(Component, Copy, Clone, Default)]
#[component(storage = "SparseSet")]
pub struct Fsparse(Mat4);

#[derive(Component, Copy, Clone, Default)]
pub struct Ftable(Mat4);

pub struct Benchmark<T> {
    world: World,
    entities: Vec<Entity>,
    _storage_type_to_test: PhantomData<T>,
}

impl<F: Component + Copy + Clone + Default> Benchmark<F> {
    pub fn new() -> Self {
        let mut world = World::default();
        let mut entities = Vec::with_capacity(10_000);
        for _ in 0..10_000 {
            entities.push(
                world
                    .spawn()
                    .insert_bundle((
                        A(Mat4::from_scale(Vec3::ONE)),
                        B(Mat4::from_scale(Vec3::ONE)),
                        C(Mat4::from_scale(Vec3::ONE)),
                        D(Mat4::from_scale(Vec3::ONE)),
                        E(Mat4::from_scale(Vec3::ONE)),
                    ))
                    .id(),
            );
        }

        Self {
            world,
            entities,
            _storage_type_to_test: Default::default(),
        }
    }

    pub fn run(&mut self) {
        for entity in &self.entities {
            self.world.entity_mut(*entity).insert(F::default());
        }

        for entity in &self.entities {
            self.world.entity_mut(*entity).remove::<F>();
        }
    }
}

pub type BenchmarkTable = Benchmark<Ftable>;
pub type BenchmarkSparse = Benchmark<Fsparse>;
