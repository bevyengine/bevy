use bevy_ecs::prelude::*;

macro_rules! create_entities {
    ($world:ident; $( $variants:ident ),*) => {
        $(
            #[derive(Component)]
            struct $variants(f32);
            for _ in 0..5 {
                $world.spawn($variants(0.0));
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
        for _ in 0..5 {
            world.spawn((
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

        create_entities!(world; C00, C01, C02, C03, C04, C05, C06, C07, C08, C09);
        create_entities!(world; C10, C11, C12, C13, C14, C15, C16, C17, C18, C19);
        create_entities!(world; C20, C21, C22, C23, C24, C25, C26, C27, C28, C29);
        create_entities!(world; C30, C31, C32, C33, C34, C35, C36, C37, C38, C39);
        create_entities!(world; C40, C41, C42, C43, C44, C45, C46, C47, C48, C49);
        create_entities!(world; C50, C51, C52, C53, C54, C55, C56, C57, C58, C59);
        create_entities!(world; C60, C61, C62, C63, C64, C65, C66, C67, C68, C69);
        create_entities!(world; C70, C71, C72, C73, C74, C75, C76, C77, C78, C79);
        create_entities!(world; C80, C81, C82, C83, C84, C85, C86, C87, C88, C89);
        create_entities!(world; C90, C91, C92, C93, C94, C95, C96, C97, C98, C99);
        let query = world.query();
        Self(world, query)
    }

    #[inline(never)]
    pub fn run(&mut self) {
        self.1.iter_mut(&mut self.0).for_each(|mut data| {
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
        });
    }
}
