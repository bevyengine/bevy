//! Prints out all touch inputs.

use bevy::{input::touch::*, prelude::*};

fn main() {
    App::new()
        .add_plugin_group(DefaultPlugins)
        .add_system(touch_event_system)
        .run();
}

fn touch_event_system(mut touch_events: EventReader<TouchInput>) {
    for event in touch_events.iter() {
        info!("{:?}", event);
    }
}
