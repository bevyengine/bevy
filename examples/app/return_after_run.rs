//! Shows how to return to the calling function after a windowed Bevy app has exited.
//!
//! In windowed *Bevy* applications, executing code below a call to `App::run()` is
//! not recommended because `App::run()` might never return.
//!
//! This example demonstrates the use of `WinitSettings::return_from_run` to
//! require that `App::run()` *does* return but this is not recommended. Be sure
//! to read the documentation on both `App::run()` and `WinitSettings::return_from_run`
//! for caveats and further details:
//!
//! - <https://docs.rs/bevy/latest/bevy/app/struct.App.html#method.run>
//! - <https://docs.rs/bevy/latest/bevy/winit/struct.WinitSettings.html#structfield.return_from_run>

use bevy::{prelude::*, window::WindowPlugin, winit::WinitSettings};

fn main() {
    println!("Running Bevy App");
    App::new()
        .insert_resource(WinitSettings {
            return_from_run: true,
            ..default()
        })
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Close the window to return to the main function".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Update, system)
        .run();
    println!("Bevy App has exited. We are back in our main function.");
}

fn system() {
    info!("Logging from Bevy App");
}
