use bevy_ecs::{prelude::*, system::SystemId};
use core::hint::black_box;
use glam::*;

const PLANETS: u16 = 1_000;
const SPAWNS: usize = 1_000_000;

#[derive(Component, Copy, Clone, PartialEq, Eq, Hash)]
#[component(immutable)]
struct Planet(u16);

fn find_planet_zeroes_indexed(query: QueryByIndex<Planet, &Planet>) {
    let mut query = query.at(&Planet(0));
    for planet in query.query().iter() {
        let _ = black_box(planet);
    }
}

pub struct Benchmark(World, SystemId);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::new();

        world.add_index(IndexOptions::<Planet>::default());

        world.spawn_batch((0..PLANETS).map(Planet).cycle().take(SPAWNS));

        let id = world.register_system(find_planet_zeroes_indexed);

        Self(world, id)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        let _ = self.0.run_system(self.1);
    }
}
