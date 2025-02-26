use bevy_ecs::prelude::*;

#[derive(Component)]
#[component(
    on_add = wrong_bazzing("foo"),
    //~^ E0001
)]
pub struct FooWrongCall;

fn wrong_bazzing(path: &str) -> impl Fn(bevy::ecs::world::DeferredWorld) {
    |world| {}
}

#[derive(Component)]
#[component(
    on_add = |w| {},
    //~^ E0001
)]
pub struct FooWrongCall;
