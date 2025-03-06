use bevy_ecs::prelude::*;

// this should fail since the function is required to have the signature
// (DeferredWorld, HookContext) -> ()
#[derive(Component)]
//~^ E0057
#[component(
    on_add = wrong_bazzing("foo"),
)]
pub struct FooWrongCall;

fn wrong_bazzing(path: &str) -> impl Fn(bevy_ecs::world::DeferredWorld) {
    |world| {}
}
