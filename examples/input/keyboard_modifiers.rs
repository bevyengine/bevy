use bevy::{
    input::{keyboard::KeyCode, Input},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_system(keyboard_input_system)
        .run();
}

/// This system prints when Ctrl + Shift + A is pressed
fn keyboard_input_system(input: Res<Input<KeyCode>>) {
    let shift = input.any_pressed([KeyCode::LShift, KeyCode::RShift]);
    let ctrl = input.any_pressed([KeyCode::LControl, KeyCode::RControl]);

    if ctrl && shift && input.just_pressed(KeyCode::A) {
        info!("Just pressed Ctrl + Shift + A!");
    }
}
