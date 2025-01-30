use bevy_ecs::{prelude::*, system::SystemId};
use core::hint::black_box;
use glam::*;

const PLANETS: u8 = 16;
const SPAWNS: usize = 10_000;

#[derive(Component, Copy, Clone, PartialEq, Eq, Hash)]
#[component(immutable)]
struct Planet(u8);

fn find_planet_zeroes_naive(query: Query<&Planet>) {
    for planet in query.iter().filter(|&&planet| planet == Planet(0)) {
        let _ = black_box(planet);
    }
}

pub struct Benchmark(World, SystemId);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::new();

        world.spawn_batch((0..PLANETS).map(Planet).cycle().take(SPAWNS));

        let id = world.register_system(find_planet_zeroes_naive);

        Self(world, id)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        let _ = self.0.run_system(self.1);
    }
}
