use bevy::{app::ScheduleRunnerPlugin, prelude::*};
use std::time::Duration;

// This example disables the default plugins by not registering them during setup.
// You can also completely remove rendering / windowing Plugin code from bevy
// by making your import look like this in your Cargo.toml
//
// [dependencies]
// bevy = { version = "*", default-features = false }
// # replace "*" with the most recent version of bevy

fn main() {
    // this app runs once
    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_once())
        .add_system(hello_world_system.system())
        .run();

    // this app loops forever at 60 fps
    App::build()
        .add_plugin(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
            1.0 / 60.0,
        )))
        .add_system(counter.system())
        .run();
}

fn hello_world_system() {
    println!("hello world");
}

fn counter(mut state: Local<CounterState>) {
    if state.count % 60 == 0 {
        println!("{}", state.count);
    }
    state.count += 1;
}

#[derive(Default)]
struct CounterState {
    count: u32,
}
