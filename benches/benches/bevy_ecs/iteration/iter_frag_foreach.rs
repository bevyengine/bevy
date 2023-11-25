use bevy_ecs::prelude::*;

macro_rules! create_entities {
    ($world:ident; $( $variants:ident ),*) => {
        $(
            #[derive(Component)]
            struct $variants(f32);
            for _ in 0..20 {
                $world.spawn(($variants(0.0), Data(1.0)));
            }
        )*
    };
}

#[derive(Component)]
struct Data(f32);

pub struct Benchmark<'w>(World, QueryState<&'w mut Data>);

impl<'w> Benchmark<'w> {
    pub fn new() -> Self {
        let mut world = World::new();

        create_entities!(world; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

        let query = world.query::<&mut Data>();
        Self(world, query)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        self.1.for_each_mut(&mut self.0, |mut data| {
            data.0 *= 2.0;
        });
    }
}
