//! This example only enables a minimal set of plugins required for bevy to run.
//! You can also completely remove rendering / windowing Plugin code from bevy
//! by making your import look like this in your Cargo.toml.
//!
//! ```toml
//! [dependencies]
//! bevy = { version = "*", default-features = false }
//! # replace "*" with the most recent version of bevy
//! ```

use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*, utils::Duration};

fn main() {
    // This app runs once
    App::new()
        .add_plugins(HeadlessPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_systems(Update, hello_world_system)
        .run();

    // This app loops forever at 60 fps
    App::new()
        .add_plugins(
            HeadlessPlugins
                .set(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(
                    1.0 / 60.0,
                )))
                // The log and ctrl+c plugin can only be registered once globally,
                // which means we need to disable it here, because it was already registered with the
                // app that runs once.
                .disable::<LogPlugin>(),
        )
        .add_systems(Update, counter)
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
