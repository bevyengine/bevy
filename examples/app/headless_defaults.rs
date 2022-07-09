//! An application that runs with default plugins, but without an actual renderer.
//! This can be very useful for integration tests or CI.

use bevy::{app::AppExit, prelude::*, render::settings::WgpuSettings, window::WindowSettings};

fn main() {
    App::new()
        .insert_resource(WgpuSettings {
            backends: None,
            ..default()
        })
        .insert_resource(WindowSettings {
            add_primary_window: false,
            exit_on_all_closed: false,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_system(do_something)
        .run();
}

// Normally Bevy exits when all windows are closed.
//
// When running in headless mode there are no windows so
// you must manually send an [`bevy::app::AppExit`] event.
fn do_something(mut app_exit_events: EventWriter<AppExit>) {
    info!("Successfully ran! Exiting...");
    app_exit_events.send(AppExit);
}
