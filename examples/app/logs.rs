use bevy::prelude::*;

/// This example illustrates how to use logs in bevy
fn main() {
    App::new()
        // Uncomment this to override the default log settings:
        // .insert_resource(bevy::log::LogSettings {
        //     level: bevy::log::Level::TRACE,
        //     filter: "wgpu=warn,bevy_ecs=info".to_string(),
        // })
        .add_plugins(DefaultPlugins)
        .add_system(log_system)
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
    // you can control what level is logged by adding the LogSettings resource
    // alternatively you can set the log level via the RUST_LOG=LEVEL environment variable
    // ex: RUST_LOG=trace, RUST_LOG=info,bevy_ecs=warn
    // the format used here is super flexible. check out this documentation for more info:
    // https://docs.rs/tracing-subscriber/*/tracing_subscriber/filter/struct.EnvFilter.html
}
