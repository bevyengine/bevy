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
//~^ ERROR: invalid attribute, expected `mutable` or `derive`
struct MutableInvalidAttribute {
    a: &'static mut Foo,
}

#[derive(QueryData)]
#[query_data(mutable(foo))]
//~^ ERROR: `mutable` does not take any arguments
struct MutableInvalidAttributeParameters {
    a: &'static mut Foo,
}

#[derive(QueryData)]
#[query_data(derive)]
//~^ ERROR: `derive` requires at least one argument
struct MutableMissingAttributeParameters {
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
