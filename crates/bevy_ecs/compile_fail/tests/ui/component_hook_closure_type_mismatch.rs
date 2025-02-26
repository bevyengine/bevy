use bevy_ecs::prelude::*;

#[derive(Component)]
//~^ E0057
#[component(
    on_add = wrong_bazzing("foo"),
)]
pub struct FooWrongCall;

fn wrong_bazzing(path: &str) -> impl Fn(bevy_ecs::world::DeferredWorld) {
    |world| {}
}

#[derive(Component)]
//~^ E0057
#[component(
    on_add = |w| {},
)]
pub struct FooWrongClosure;
