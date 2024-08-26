use bevy_ecs::prelude::*;
use glam::*;

#[derive(Component, Copy, Clone)]
struct Transform(Mat4);

#[derive(Component, Copy, Clone)]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Rotation(Vec3);

#[derive(Component, Copy, Clone)]
struct Velocity(Vec3);

pub struct Benchmark(World, Box<dyn System<In = (), Out = ()>>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::new();

        world.spawn_batch(
            std::iter::repeat((
                Transform(Mat4::from_scale(Vec3::ONE)),
                Position(Vec3::X),
                Rotation(Vec3::X),
                Velocity(Vec3::X),
            ))
            .take(10_000),
        );

        fn query_system(mut query: Query<(&Velocity, &mut Position)>) {
            for (velocity, mut position) in &mut query {
                position.0 += velocity.0;
            }
        }

        let mut system = IntoSystem::into_system(query_system);
        system.initialize(&mut world);
        system.update_archetype_component_access(world.as_unsafe_world_cell());
        Self(world, Box::new(system))
    }

    #[inline(never)]
    pub fn run(&mut self) {
        self.1.run((), &mut self.0);
    }
}
