use bevy::prelude::*;

#[derive(Component)]
struct A(f32);
#[derive(Component)]
struct B(f32);

pub struct Benchmark(World, Vec<Entity>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::default();

        let entities = world
            .spawn_batch((0..10000).map(|_| A(0.0)))
            .collect::<Vec<_>>();

        Self(world, entities)
    }

    pub fn run(&mut self) {
        for entity in &self.1 {
            self.0.insert_one(*entity, B(0.0)).unwrap();
        }

        for entity in &self.1 {
            self.0.remove_one::<B>(*entity).unwrap();
        }
    }
}
