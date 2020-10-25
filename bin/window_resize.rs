use bevy::prelude::*;
use bevy::window::{WindowResized};

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(window_resize.system())
        .run();
}

fn window_resize(resize_event: Res<Events<WindowResized>>) {
    let mut event_reader = resize_event.get_reader();
    for event in event_reader.iter(&resize_event) {
        println!("Info: {:?}",event);
    }
}