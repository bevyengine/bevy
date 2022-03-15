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

pub struct Benchmark(World, Box<dyn System<In = (), Out = ()>>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::new();

        // TODO: batch this
        for _ in 0..10_000 {
            world.spawn().insert_bundle((
                Transform(Matrix4::from_scale(1.0)),
                Position(Vector3::unit_x()),
                Rotation(Vector3::unit_x()),
                Velocity(Vector3::unit_x()),
            ));
        }

        fn query_system(mut query: Query<(&Velocity, &mut Position)>) {
            for (velocity, mut position) in query.iter_mut() {
                position.0 += velocity.0;
            }
        }

        let mut system = query_system.system();
        system.initialize(&mut world);
        for archetype in world.archetypes().iter() {
            system.new_archetype(archetype);
        }
        Self(world, Box::new(system))
    }

    pub fn run(&mut self) {
        self.1.run((), &mut self.0);
    }
}
