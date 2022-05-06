use bevy_ecs::prelude::*;
use std::cell::Cell;

#[derive(Component, Eq, PartialEq, Debug)]
struct A(Cell<usize>);

fn main() {
    let mut world = World::default();
    let e = world.spawn().insert(A(Cell::new(10_usize))).id();

    {
        let query = QueryState::<&A>::new(&mut world);
    }

    {
        let value = world.get::<A>(e);
    }

    {
        let value = world.entity(e).get::<A>();
    }

    {
        let value = world.entity_mut(e).get::<A>();
    }
}
