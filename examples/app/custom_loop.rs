//! This example demonstrates you can create a custom runner (to update an app manually). It reads
//! lines from stdin and prints them from within the ecs.

use bevy::{app::AppExit, prelude::*};
use std::io;

#[derive(Resource)]
struct Input(String);

fn my_runner(mut app: App) -> AppExit {
    // Finalize plugin building, including running any necessary clean-up.
    // This is normally completed by the default runner.
    app.finish();
    app.cleanup();

    println!("Type stuff into the console");
    for line in io::stdin().lines() {
        {
            let mut input = app.world_mut().resource_mut::<Input>();
            input.0 = line.unwrap();
        }
        app.update();

        if let Some(exit) = app.should_exit() {
            return exit;
        }
    }

    AppExit::Success
}

fn print_system(input: Res<Input>) {
    println!("You typed: {}", input.0);
}

fn exit_system(input: Res<Input>, mut exit_event: EventWriter<AppExit>) {
    if input.0 == "exit" {
        exit_event.send(AppExit::Success);
    }
}

// AppExit implements `Termination` so we can return it from main.
fn main() -> AppExit {
    App::new()
        .insert_resource(Input(String::new()))
        .set_runner(my_runner)
        .add_systems(Update, (print_system, exit_system))
        .run()
}
