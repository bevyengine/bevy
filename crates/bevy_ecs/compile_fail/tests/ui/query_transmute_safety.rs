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
        let mut lens_a = query.transmute_lens::<&mut Foo>();
        let mut lens_b = query.transmute_lens::<&mut Foo>();
        //~^ E0499

        let mut query_a = lens_a.query();
        let mut query_b = lens_b.query();

        let a = query_a.single_mut();
        let b = query_b.single_mut(); // oops 2 mutable references to same Foo
        assert_eq!(*a, *b);
    }

    {
        let mut lens = query.transmute_lens::<&mut Foo>();

        let mut query_a = lens.query();
        let mut query_b = lens.query();
        //~^ E0499

        let a = query_a.single_mut();
        let b = query_b.single_mut(); // oops 2 mutable references to same Foo
        assert_eq!(*a, *b);
    }
}
