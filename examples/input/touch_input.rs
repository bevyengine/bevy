use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(touch_system.system())
        .run();
}

// This system prints messages when you use the touchscreen
fn touch_system(finger_input: Res<Input<Finger>>) {
    if finger_input.pressed(Finger(0)) {
        println!("finger 0 pressed");
    }

    if finger_input.just_pressed(Finger(0)) {
        println!("finger 0 just pressed");
    }

    if finger_input.just_released(Finger(0)) {
        println!("finger 0 just released");
    }
}
