use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(f32);

pub struct Benchmark(World, Entity, Box<dyn System<In = Entity, Out = ()>>);

impl Benchmark {
    pub fn new() -> Self {
        let mut world = World::new();

        let entity = world.spawn().insert(A(0.0)).id();
        fn query_system(In(entity): In<Entity>, mut query: Query<&mut A>) {
            for _ in 0..100_000 {
                let mut a = query.get_mut(entity).unwrap();
                a.0 += 1.0;
            }
        }

        let mut system = IntoSystem::into_system(query_system);
        system.initialize(&mut world);
        system.update_archetype_component_access(&world);
        Self(world, entity, Box::new(system))
    }

    pub fn run(&mut self) {
        self.2.run(self.1, &mut self.0);
    }
}
