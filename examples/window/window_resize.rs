//! This example illustrates how to handle window resize events and fit the window

use bevy::input::system::exit_on_esc_system;
use bevy::prelude::*;
use bevy::window::WindowResized;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(on_resize_system)
        .add_system(exit_on_esc_system)
        .run();
}

/// The system iterates resize events and print them.
pub fn on_resize_system(mut resize_reader: EventReader<WindowResized>) {
    for e in resize_reader.iter() {
        println!("event {:?}", e);
    }
}
