use bevy_ecs::prelude::*;

#[derive(Component)]
#[component(
    on_add = Bar,
    //~^ E0001
)]
pub struct FooWrongPath;
