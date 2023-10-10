use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemState;

#[derive(Component, Eq, PartialEq, Debug)]
struct Foo(u32);

#[derive(Component)]
struct Bar;

fn main() {
    let mut world = World::default();
    world.spawn(Foo(10));

    let mut system_state = SystemState::<Query<(&mut Foo, &Bar)>>::new(&mut world);
    let mut query = system_state.get_mut(&mut world);

    {
        let mut lens1 = query.transmute_fetch::<&mut Foo>();
        let mut lens2 = query.transmute_fetch::<&mut Foo>();

        let mut query1 = lens1.query();
        let mut query2 = lens2.query();

        let f1 = query1.single_mut();
        let f2 = query2.single_mut(); // oops 2 mutable references to same Foo
        assert_eq!(*f1, *f2);
    }

    {
        let mut lens = query.transmute_fetch::<&mut Foo>();

        let mut query1 = lens.query();
        let mut query2 = lens.query();

        let f1 = query1.single_mut();
        let f2 = query2.single_mut(); // oops 2 mutable references to same Foo
        assert_eq!(*f1, *f2);
    }
}
