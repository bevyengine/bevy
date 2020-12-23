use bevy::{input::touch::*, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(touch_system.system())
        .run();
}

fn touch_system(touches: Res<Touches>) {
    for touch in touches.iter_just_pressed() {
        println!(
            "just pressed touch with id: {:?}, at: {:?}",
            touch.id(),
            touch.position()
        );
    }

    for touch in touches.iter_just_released() {
        println!(
            "just released touch with id: {:?}, at: {:?}",
            touch.id(),
            touch.position()
        );
    }

    for touch in touches.iter_just_cancelled() {
        println!("cancelled touch with id: {:?}", touch.id());
    }

    // you can also iterate all current touches and retrieve their state like this:
    for touch in touches.iter() {
        println!("active touch: {:?}", touch);
        println!("  just_pressed: {}", touches.just_pressed(touch.id()));
    }
}
