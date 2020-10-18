use bevy::{input::touch::*, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(touch_system.system())
        .run();
}

fn touch_system(touches: Res<Touches>) {
    for touch in touches.iter_just_pressed() {
        println!(
            "just pressed touch with id: {:?}, at: {:?}",
            touch.id, touch.position
        );
    }

    for touch in touches.iter_just_released() {
        println!(
            "just released touch with id: {:?}, at: {:?}",
            touch.id, touch.position
        );
    }

    for touch in touches.iter_just_cancelled() {
        println!("cancelled touch with id: {:?}", touch.id);
    }
}
