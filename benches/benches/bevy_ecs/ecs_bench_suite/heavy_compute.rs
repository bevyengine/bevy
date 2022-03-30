use bevy_ecs::prelude::*;
use bevy_tasks::TaskPool;
use glam::*;

#[derive(Component, Copy, Clone)]
struct Position(Vec3);

#[derive(Component, Copy, Clone)]
struct Rotation(Vec3);

#[derive(Component, Copy, Clone)]
struct Velocity(Vec3);

#[derive(Component, Copy, Clone)]
struct Transform(Mat4);

pub struct Benchmark(World, Box<dyn System<In = (), Out = ()>>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::default();

        world.spawn_batch((0..1000).map(|_| {
            (
                Transform(Mat4::from_axis_angle(Vec3::X, 1.2)),
                Position(Vec3::X),
                Rotation(Vec3::X),
                Velocity(Vec3::X),
            )
        }));

        fn sys(task_pool: Res<TaskPool>, mut query: Query<(&mut Position, &mut Transform)>) {
            query.par_for_each_mut(&task_pool, 128, |(mut pos, mut mat)| {
                for _ in 0..100 {
                    mat.0 = mat.0.inverse();
                }

                pos.0 = mat.0.transform_vector3(pos.0);
            });
        }

        world.insert_resource(TaskPool::default());
        let mut system = IntoSystem::into_system(sys);
        system.initialize(&mut world);
        system.update_archetype_component_access(&world);

        Self(world, Box::new(system))
    }

    pub fn run(&mut self) {
        self.1.run((), &mut self.0);
    }
}
