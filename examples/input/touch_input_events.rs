use bevy::{input::touch::*, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(touch_event_system.system())
        .run();
}

fn touch_event_system(mut touch_events: EventReader<TouchInput>) {
    for event in touch_events.iter() {
        println!("{:?}", event);
    }
}
