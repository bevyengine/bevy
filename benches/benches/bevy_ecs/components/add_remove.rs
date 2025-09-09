use bevy_ecs::prelude::*;

#[derive(Component, Clone)]
struct A(f32);
#[derive(Component)]
struct B(f32);

pub struct Benchmark(World, Vec<Entity>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::default();

        let entities = world
            .spawn_batch(core::iter::repeat_n(A(0.), 10_000))
            .collect();
        Self(world, entities)
    }

    pub fn run(&mut self) {
        for entity in &self.1 {
            self.0.entity_mut(*entity).insert(B(0.));
        }

        for entity in &self.1 {
            self.0.entity_mut(*entity).remove::<B>();
        }
    }
}
