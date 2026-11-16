use bevy_ecs::{bundle::NoBundleEffect, prelude::*, spawn::SpawnOneRelated};

#[derive(Bundle)]
#[bundle(ignore_from_components)]
struct A(SpawnOneRelated<ChildOf, ()>);

#[derive(Bundle)]
struct B;

fn main() {
    // bundle with side-effects should not implement NoBundleEffect.
    bundle_has_no_effects::<A>();
    //~^ E0277

    // this should not fail: B has no side-effects.
    bundle_has_no_effects::<B>();
}

fn bundle_has_no_effects<T: Bundle<Effect: NoBundleEffect>>() {}
