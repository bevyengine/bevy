//! Demonstrates how and why to use run criteria to control whether or not systems run

/*
    Basically, run criteria is the way to make a function run under certaion conditons.
    It can be used everywhere and it's very useful.
    For example, i have a multiplayer game that i want to run the server and the client separately, i can create two plugins, one for the server
    and one for the client and each one runs a different run criteria, for example the server runs run_server criteria and the client runs run_client criteria.

    Its really good to list program task so the task runs only when the user needs it!
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
                // You can combine run criteria
                // In this case, the system controlled by this run criteria is only evaluated a single time
                .with_run_criteria(ShouldRun::once)
                .with_system(text),
        )
        .run();
}
