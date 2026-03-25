//! This example shows how to configure the `ScheduleRunnerPlugin` to run your
//! application without windowing. You can completely remove rendering / windowing
//! Plugin code from bevy by making your import look like this in your Cargo.toml.
//!
//! ```toml
//! [dependencies]
//! bevy = { version = "*", default-features = false }
//! # replace "*" with the most recent version of bevy
//! ```
//!
//! And then enabling the features you need.
//! See the full list: <https://docs.rs/bevy/latest/bevy/#cargo-features>
use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, prelude::*};
use core::time::Duration;

fn main() {
    if cfg!(feature = "bevy_window") {
        println!("This example is running with the bevy_window feature enabled and will not run headless.");
        println!("Disable the default features and rerun the example to run headless.");
        println!("To do so, run:");
        println!();
        println!("    cargo run --example headless --no-default-features --features bevy_log");
        return;
    }

    // This app runs once
    App::new()
        .add_plugins(DefaultPlugins.set(ScheduleRunnerPlugin::run_once()))
        .add_systems(Update, hello_world_system)
        .run();

    // This app loops forever at 60 fps
    App::new()
        .add_plugins(
            DefaultPlugins
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
    if state.count.is_multiple_of(60) {
        println!("{}", state.count);
    }
    state.count += 1;
}

#[derive(Default)]
struct CounterState {
    count: u32,
}
