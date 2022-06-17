//! Prints out all touch inputs.

use bevy::{input::touch::*, prelude::*};

#[bevy_main]
async fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .await
        .add_system(touch_event_system)
        .run();
}

fn touch_event_system(mut touch_events: EventReader<TouchInput>) {
    for event in touch_events.iter() {
        info!("{:?}", event);
    }
}
