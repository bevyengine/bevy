use bevy_ecs::prelude::*;

//~v E0080
#[derive(Component)]
struct A{bad:i32}

#[derive(Component)]
#[require(A)]
struct B;