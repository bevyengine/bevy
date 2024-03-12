use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;

#[derive(Component)]
struct Foo;

#[derive(QueryData)]
struct MutableUnmarked {
    a: &'static mut Foo,
}

#[derive(QueryData)]
#[query_data(mutable)]
struct MutableMarked {
    a: &'static mut Foo,
}

#[derive(QueryData)]
struct NestedMutableUnmarked {
    a: MutableMarked,
}

fn main() {}
