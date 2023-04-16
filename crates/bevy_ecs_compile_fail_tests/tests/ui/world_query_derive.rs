use bevy_ecs::prelude::*;
use bevy_ecs::query::WorldQuery;

#[derive(Component)]
struct Foo;

#[derive(WorldQuery)]
struct MutableUnmarked {
    a: &'static mut Foo,
}

#[derive(WorldQuery)]
#[world_query(mutable)]
struct MutableMarked {
    a: &'static mut Foo,
}

#[derive(WorldQuery)]
struct NestedMutableUnmarked {
    a: MutableMarked,
}

fn main() {}
