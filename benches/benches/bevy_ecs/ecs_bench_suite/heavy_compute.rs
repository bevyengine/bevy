use bevy_ecs::prelude::*;
use bevy_tasks::{
    TaskPool, ParallelIterator,
};
use cgmath::*;
use cgmath::Transform;

#[derive(Component, Copy, Clone)]
struct Position(Vector3<f32>);

#[derive(Component, Copy, Clone)]
struct Rotation(Vector3<f32>);

#[derive(Component, Copy, Clone)]
struct Velocity(Vector3<f32>);

pub struct Benchmark(World, Box<dyn System<In = (), Out = ()>>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::default();

        world.spawn_batch((0..1000).map(|_| {
            (
                Matrix4::<f32>::from_angle_x(Rad(1.2)),
                Position(Vector3::unit_x()),
                Rotation(Vector3::unit_x()),
                Velocity(Vector3::unit_x()),
            )
        }));

        fn sys(task_pool: Res<TaskPool>, mut query: Query<(&mut Position, &mut Matrix4<f32>)>) {
            query
                .par_for_each_mut(&task_pool, 128, |(mut pos, mut mat)| {
                    for _ in 0..100 {
                        *mat = mat.invert().unwrap();
                    }

                    pos.0 = mat.transform_vector(pos.0);
                });
        }

        world.insert_resource(TaskPool::default());
        let mut system = sys;
        system.initialize(&mut world);
        for archetype in world.archetypes().iter() {
            system.new_archetype(archetype);
        }

        Self(world, Box::new(system))
    }

    pub fn run(&mut self) {
        self. 1.run((), &mut self.0);
    }
}
