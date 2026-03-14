use bevy_ecs::prelude::*;
use bevy_ecs::query::QueryData;

#[derive(Component)]
struct Foo;

#[derive(QueryData)]
struct MutableUnmarked {
    //~v E0277
    a: &'static mut Foo,
}

#[derive(QueryData)]
#[query_data(mut)]
//~^ ERROR: invalid attribute, expected `mutable`, `derive` or `contiguous`
struct MutableInvalidAttribute {
    a: &'static mut Foo,
}

#[derive(QueryData)]
#[query_data(mutable)]
struct MutableMarked {
    a: &'static mut Foo,
}

#[derive(QueryData)]
struct NestedMutableUnmarked {
    //~v E0277
    a: MutableMarked,
}
