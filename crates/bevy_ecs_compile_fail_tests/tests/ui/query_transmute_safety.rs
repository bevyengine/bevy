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
        let mut subquery_a = query.subquery::<&mut Foo>();
        let mut subquery_b = query.subquery::<&mut Foo>();

        let mut query_a = subquery_a.query();
        let mut query_b = subquery_b.query();

        let a = query_a.single_mut();
        let b = query_b.single_mut(); // oops 2 mutable references to same Foo
        assert_eq!(*a, *b);
    }

    {
        let mut subquery = query.subquery::<&mut Foo>();

        let mut query_a = subquery.query();
        let mut query_b = subquery.query();

        let a = query_a.single_mut();
        let b = query_b.single_mut(); // oops 2 mutable references to same Foo
        assert_eq!(*a, *b);
    }
}
