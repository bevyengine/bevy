//! Demonstrates a startup system (one that runs once when the app starts up).

use bevy::prelude::*;

fn main() {
    App::new()
        .add_systems(Startup, startup_system)
        .add_systems(Update, normal_system)
        .run();
}

/// Startup systems are run exactly once when the app starts up.
/// They run right before "normal" systems run.
fn startup_system() {
    println!("startup system ran first");
}

fn normal_system() {
    println!("normal system ran second");
}
