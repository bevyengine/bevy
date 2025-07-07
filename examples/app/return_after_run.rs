//! Shows how to return to the calling function after a windowed Bevy app has exited.
//!
//! In windowed *Bevy* applications, executing code below a call to `App::run()` is
//! not recommended because:
//! - `App::run()` will never return on iOS and Web.
//! - It is not possible to recreate a window after the event loop has been terminated.

use bevy::{prelude::*, window::PrimaryWindow};

fn main() {
    println!("Running Bevy App");
    App::new()
        .add_plugins(DefaultPlugins)
        .add_observer(configure_window)
        .add_systems(Update, system)
        .run();
    println!("Bevy App has exited. We are back in our main function.");
}

fn configure_window(trigger: On<Add, PrimaryWindow>, mut window: Query<&mut Window>) {
    let mut window = window.get_mut(trigger.target()).unwrap();
    window.title = "Close the window to return to the main function".into();
}

fn system() {
    info!("Logging from Bevy App");
}
