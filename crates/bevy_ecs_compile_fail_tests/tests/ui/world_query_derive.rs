use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQueryData;

#[derive(Component)]
struct Foo;

#[derive(WorldQueryData)]
struct MutableUnmarked {
    a: &'static mut Foo,
}

#[derive(WorldQueryData)]
#[world_query_data(mutable)]
struct MutableMarked {
    a: &'static mut Foo,
}

#[derive(WorldQueryData)]
struct NestedMutableUnmarked {
    a: MutableMarked,
}

fn main() {}
