//! This example illustrates how to use logs in bevy.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            // Uncomment this to override the default log settings:
            // level: bevy::log::Level::TRACE,
            // filter: "wgpu=warn,bevy_ecs=info".to_string(),
            ..default()
        }))
        .add_systems(Update, log_system)
        .run();
}

fn log_system() {
    // here is how you write new logs at each "log level" (in "least important" to "most important"
    // order)
    trace!("very noisy");
    debug!("helpful for debugging");
    info!("helpful information that is worth printing by default");
    warn!("something bad happened that isn't a failure, but thats worth calling out");
    error!("something failed");

    // by default, trace and debug logs are ignored because they are "noisy"
    // you can control what level is logged by setting up the LogPlugin
    // alternatively you can set the log level via the RUST_LOG=LEVEL environment variable
    // ex: RUST_LOG=trace, RUST_LOG=info,bevy_ecs=warn
    // the format used here is super flexible. check out this documentation for more info:
    // https://docs.rs/tracing-subscriber/*/tracing_subscriber/filter/struct.EnvFilter.html
}
