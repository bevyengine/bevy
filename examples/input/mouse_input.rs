use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(mouse_click_system.system())
        .run();
}

// This system prints messages when you press or release the left mouse button:
fn mouse_click_system(mouse_button_input: Res<BinaryInput<MouseButtonCode>>) {
    if mouse_button_input.pressed(MouseButtonCode::Left) {
        println!("left mouse currently pressed");
    }

    if mouse_button_input.just_pressed(MouseButtonCode::Left) {
        println!("left mouse just pressed");
    }

    if mouse_button_input.just_released(MouseButtonCode::Left) {
        println!("left mouse just released");
    }
}
