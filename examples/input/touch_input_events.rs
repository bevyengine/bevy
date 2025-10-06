//! Prints out all touch inputs.

use bevy::{input::touch::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, touch_event_system)
        .run();
}

fn touch_event_system(mut touch_inputs: MessageReader<TouchInput>) {
    for touch_input in touch_inputs.read() {
        info!("{:?}", touch_input);
    }
}
