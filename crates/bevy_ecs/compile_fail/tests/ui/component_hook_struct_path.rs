use bevy_ecs::prelude::*;

#[derive(Component)]
#[component(
    on_add = Bar,
    //~^ E0425
)]
pub struct FooWrongPath;
