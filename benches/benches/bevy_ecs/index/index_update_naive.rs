use bevy_ecs::{prelude::*, system::SystemId};
use glam::*;

const PLANETS: u8 = 16;
const SPAWNS: usize = 10_000;

#[derive(Component, Copy, Clone, PartialEq, Eq, Hash)]
#[component(immutable)]
struct Planet(u8);

fn increment_planet_zeroes_naive(
    query: Query<(Entity, &Planet)>,
    mut local: Local<u8>,
    mut commands: Commands,
) {
    let target = Planet(*local);
    let next_planet = Planet(target.0 + 1);

    for (entity, _planet) in query.iter().filter(|(_, planet)| **planet == target) {
        commands.entity(entity).insert(next_planet);
    }

    *local += 1;
}

pub struct Benchmark(World, SystemId);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::new();

        world.spawn_batch((0..PLANETS).map(Planet).cycle().take(SPAWNS));

        let id = world.register_system(increment_planet_zeroes_naive);

        Self(world, id)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        let _ = self.0.run_system(self.1);
        self.0.flush();
    }
}
