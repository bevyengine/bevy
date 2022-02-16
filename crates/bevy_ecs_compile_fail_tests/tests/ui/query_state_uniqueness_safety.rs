use bevy_ecs::prelude::*;

#[derive(Component, Eq, PartialEq, Debug)]
struct A(usize);

fn main() {
    let mut world = World::default();
    world.spawn().insert(A(1));

    let first_query_state: QueryState<&mut A> = world.query();
    let second_query_state: QueryState<&mut A> = world.query();

    let mut first_query = Query::from_state(&mut world, &first_query_state);
    // This should fail to compile, as another query is already active
    let mut second_query = Query::from_state(&mut world, &second_query_state);

    // This is a clear violation of no-aliased mutability
    assert_eq!(*first_query.single_mut(), *second_query.single_mut());
}
