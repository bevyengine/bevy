use bevy_ecs::prelude::*;

#[derive(Component)]
struct A {
    bad: i32,
}

//~v E0080
#[derive(Component)]
#[require(A)]
struct B;
