use bevy_ecs::prelude::*;

#[derive(Resource)]
//~v ERROR: Lifetimes must be 'static
struct A<'a> {
    foo: &'a str,
}

#[derive(Resource)]
struct B<'a: 'static> {
    foo: &'a str,
}
