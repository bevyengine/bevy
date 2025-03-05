use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;

#[derive(Component, Eq, PartialEq, Debug)]
struct Foo(u32);

fn main() {
    let mut world = World::default();
    let e = world.spawn(Foo(10_u32)).id();

    let mut system_state = SystemState::<Query<&mut Foo>>::new(&mut world);
    {
        let mut query = system_state.get_mut(&mut world);
        let mut lens = query.as_query_lens();
        dbg!("hi");
        {
            let mut data: Mut<Foo> = lens.query().get_inner(e).unwrap();
            let mut data2: Mut<Foo> = lens.query().get_inner(e).unwrap();
            //~^ E0499
            assert_eq!(&mut *data, &mut *data2); // oops UB
        }
        dbg!("bye");
    }
}
