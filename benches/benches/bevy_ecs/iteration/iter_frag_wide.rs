use bevy_ecs::prelude::*;

macro_rules! create_entities {
    ($world:ident; $( $variants:ident ),*) => {
        $(
            #[derive(Component)]
            struct $variants(f32);
            for _ in 0..20 {
                $world.spawn((
                    $variants(0.0),
                    Data::<0>(1.0),
                    Data::<1>(1.0),
                    Data::<2>(1.0),
                    Data::<3>(1.0),
                    Data::<4>(1.0),
                    Data::<5>(1.0),
                    Data::<6>(1.0),
                    Data::<7>(1.0),
                    Data::<8>(1.0),
                    Data::<9>(1.0),
                    Data::<10>(1.0),
                ));
            }
        )*
    };
}

#[derive(Component)]
struct Data<const X: usize>(f32);

pub struct Benchmark<'w>(
    World,
    QueryState<(
        &'w mut Data<0>,
        &'w mut Data<1>,
        &'w mut Data<2>,
        &'w mut Data<3>,
        &'w mut Data<4>,
        &'w mut Data<5>,
        &'w mut Data<6>,
        &'w mut Data<7>,
        &'w mut Data<8>,
        &'w mut Data<9>,
        &'w mut Data<10>,
    )>,
);

impl<'w> Benchmark<'w> {
    pub fn new() -> Self {
        let mut world = World::new();

        create_entities!(world; A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W, X, Y, Z);

        let query = world.query();
        Self(world, query)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        for mut data in self.1.iter_mut(&mut self.0) {
            data.0 .0 *= 2.0;
            data.1 .0 *= 2.0;
            data.2 .0 *= 2.0;
            data.3 .0 *= 2.0;
            data.4 .0 *= 2.0;
            data.5 .0 *= 2.0;
            data.6 .0 *= 2.0;
            data.7 .0 *= 2.0;
            data.8 .0 *= 2.0;
            data.9 .0 *= 2.0;
            data.10 .0 *= 2.0;
        }
    }
}
