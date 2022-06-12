use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQueryFilter;

#[derive(Component)]
struct Foo;

#[derive(WorldQueryFilter)]
struct ComponentQuery {
    a: &'static Foo,
}

fn main() {}
