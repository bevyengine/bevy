use bevy_ecs::prelude::*;

#[derive(Component)]
struct A(f32);

pub struct Benchmark<'w>(World, Entity, QueryState<&'w mut A>);

impl<'w> Benchmark<'w> {
    pub fn new() -> Self {
        let mut world = World::new();

        let entity = world.spawn().insert(A(0.0)).id();
        let query = world.query::<&mut A>();
        Self(world, entity, query)
    }

    pub fn run(&mut self) {
        for _x in 0..100000 {
            let mut a = unsafe { self.2.get_unchecked(&mut self.0, self.1).unwrap() };
            a.0 += 1.0;
        }
    }
}
