//! This example illustrates how to use logs in bevy.

use bevy::log::once;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(bevy::log::LogPlugin {
            // Uncomment this to override the default log settings:
            // level: bevy::log::Level::TRACE,
            // filter: "wgpu=warn,bevy_ecs=info".to_string(),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, log_system)
        .add_systems(Update, log_once_system)
        .add_systems(Update, panic_on_p)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn(TextBundle {
        text: Text::from_section("Press P to panic", TextStyle::default()),
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ..default()
    });
}

fn panic_on_p(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::KeyP) {
        panic!("P pressed, panicking");
    }
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

fn log_once_system() {
    // The 'once' variants of each log level are useful when a system is called every frame,
    // but we still wish to inform the user only once. In other words, use these to prevent spam :)

    trace_once!("one time noisy message");
    debug_once!("one time debug message");
    info_once!("some info which is printed only once");
    warn_once!("some warning we wish to call out only once");
    error_once!("some error we wish to report only once");

    for i in 0..10 {
        info_once!("logs once per call site, so this works just fine: {}", i);
    }

    // you can also use the `once!` macro directly,
    // in situations where you want to do something expensive only once
    // within the context of a continuous system.
    once!({
        info!("doing expensive things");
        let mut a: u64 = 0;
        for i in 0..100000000 {
            a += i;
        }
        info!("result of some expensive one time calculation: {}", a);
    });
}
