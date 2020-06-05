use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_system(mouse_click_system.system())
        .run();
}

// This system prints messages when you press or release the left mouse button:
fn mouse_click_system(mouse_button_input: Res<Input<MouseButton>>) {
    if mouse_button_input.just_pressed(MouseButton::Left) {
        println!("left mouse clicked");
    }

    if mouse_button_input.just_released(MouseButton::Left) {
        println!("left mouse released");
    }
}
