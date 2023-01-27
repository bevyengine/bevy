//! Demonstrates how and why to use run criteria to control whether or not systems run

/*
    Basically, run criteria is the way to make a function run under certaion conditons.
    It can be used everywhere and it's very useful.
*/
use bevy::{ecs::schedule::ShouldRun, prelude::*};

// The criteria system
fn run_if() -> ShouldRun {
    // X value to start the criteria
    let x = 2;

    if x > 1 {
        ShouldRun::Yes
    } else {
        ShouldRun::No
    }
}

// Simple print function.
fn text(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((TextBundle::from_sections([TextSection::new(
        "Running...",
        TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 60.0,
            color: Color::WHITE,
        },
    )]),));
}

// Simple 2D camera
fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

// Main function
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(run_if)
                // Another major criteria(specifically for this example) to run the criteria only once
                .with_run_criteria(ShouldRun::once)
                .with_system(text),
        )
        .run();
}
