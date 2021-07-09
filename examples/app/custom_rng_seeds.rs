use bevy::prelude::*;

/// This example illustrates how to customize the random number generator seeds.
fn main() {
    // These are not very good seeds!
    let seed_with_ones: [u8; 32] = [1; 32];
    let seed_with_twos: [u8; 32] = [2; 32];

    App::build()
        .insert_resource(DefaultRngOptions::with_seeds(
            SecureSeed::from(seed_with_ones),
            InsecureSeed::from(seed_with_twos),
        ))
        .add_plugins(DefaultPlugins)
        .run();
}
