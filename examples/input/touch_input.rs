use bevy::{input::touch::*, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(touch_system.system())
        .run();
}

fn touch_system(touches: Res<Touches>) {
    for touch in touches.iter() {
        println!(
            "active touch: {} {} {} {}",
            touch.id, touch.position, touch.previous_position, touch.start_position
        );

        if touches.just_pressed(touch.id) {
            println!(
                "just pressed touch with id: {:?}, at: {:?}",
                touch.id, touch.position
            );
        }

        if touches.just_released(touch.id) {
            println!(
                "just released touch with id: {:?}, at: {:?}",
                touch.id, touch.position
            );
        }

        if touches.just_cancelled(touch.id) {
            println!("cancelled touch with id: {:?}", touch.id);
        }
    }
}
