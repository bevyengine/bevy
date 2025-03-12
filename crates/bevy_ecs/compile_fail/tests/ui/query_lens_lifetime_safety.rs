use bevy_ecs::prelude::*;
use bevy_ecs::system::{QueryLens, SystemState};

#[derive(Component, Eq, PartialEq, Debug)]
struct Foo(u32);

#[derive(Component, Eq, PartialEq, Debug)]
struct Bar(u32);

fn main() {
    let mut world = World::default();
    let e = world.spawn((Foo(10_u32), Bar(10_u32))).id();

    let mut system_state = SystemState::<(Query<&mut Foo>, Query<&mut Bar>)>::new(&mut world);
    {
        let (mut foo_query, mut bar_query) = system_state.get_mut(&mut world);
        dbg!("hi");
        {
            let mut lens = foo_query.as_query_lens();
            let mut data: Mut<Foo> = lens.query().get_inner(e).unwrap();
            let mut data2: Mut<Foo> = lens.query().get_inner(e).unwrap();
            //~^ E0499
            assert_eq!(&mut *data, &mut *data2); // oops UB
        }

        {
            let mut join: QueryLens<(&mut Foo, &mut Bar)> = foo_query.join(&mut bar_query);
            let mut query = join.query();
            let (_, mut data) = query.single_mut().unwrap();
            let mut data2 = bar_query.single_mut().unwrap();
            //~^ E0499
            assert_eq!(&mut *data, &mut *data2); // oops UB
        }

        {
            let mut join: QueryLens<(&mut Foo, &mut Bar)> =
                foo_query.join_inner(bar_query.reborrow());
            let mut query = join.query();
            let (_, mut data) = query.single_mut().unwrap();
            let mut data2 = bar_query.single_mut().unwrap();
            //~^ E0499
            assert_eq!(&mut *data, &mut *data2); // oops UB
        }
        dbg!("bye");
    }
}
