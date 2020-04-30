use bevy::{
    app::schedule_runner::{RunMode, ScheduleRunnerPlugin},
    prelude::*,
};
use std::time::Duration;

// This example disables the default plugins by not registering them during setup.
// You can also completely remove rendering / windowing Plugin code from bevy
// by making your import look like this in your Cargo.toml
//
// [dependencies]
// bevy = { version = "0.1.0", default-features = false, features = ["headless"] }

fn main() {
    // this app runs once
    App::build()
        .add_plugin(ScheduleRunnerPlugin {
            run_mode: RunMode::Once,
        })
        .add_system(hello_world_system.into_system("hello"))
        .run();

    // this app loops forever at 60 fps
    App::build()
        .add_plugin(ScheduleRunnerPlugin {
            run_mode: RunMode::Loop {
                wait: Some(Duration::from_secs_f64(1.0 / 60.0)),
            },
        })
        .add_system(hello_world_system.into_system("hello"))
        .run();
}

pub fn hello_world_system() {
    println!("hello world");
}
