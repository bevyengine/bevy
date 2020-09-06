use bevy::prelude::*;
use bevy_app::AppExit;
use bevy_prototype_input_map::{
    axis::Axis, inputmap::InputMap, keyboard::KeyboardMap, mouse::MouseMap,
};

fn main() {
    App::build()
        .add_default_plugins()
        // setup
        .add_plugin(bevy_prototype_input_map::InputMapPlugin::default())
        .add_startup_system(setup.system())
        .add_system(action_system.system())
        .run();
}

fn setup(
    mut key_map: ResMut<KeyboardMap>,
    mut mouse_map: ResMut<MouseMap>,
    mut input_map: ResMut<InputMap>,
) {
    // keyboard
    key_map.bind_keyboard_pressed(KeyCode::Space, "JUMP".to_string());
    key_map.bind_keyboard_pressed(KeyCode::Return, "SHOOT".to_string());

    key_map.bind_keyboard_pressed(KeyCode::Escape, "QUIT_APP".to_string());

    // mouse
    mouse_map.bind_mouse_button_pressed(MouseButton::Left, "SHOOT".to_string());
    mouse_map.bind_mouse_button_pressed(MouseButton::Right, "JUMP".to_string());

    mouse_map.bind_mouse_motion(Axis::Y_Negative, "AIM_UP".to_string());
    mouse_map.bind_mouse_motion(Axis::Y_Positive, "AIM_DOWN".to_string());
    mouse_map.bind_mouse_motion(Axis::X_Negative, "AIM_LEFT".to_string());
    mouse_map.bind_mouse_motion(Axis::X_Positive, "AIM_RIGHT".to_string());

    // input map
    // meaningful only for analog inputs like mouse move, joystick...etc
    input_map.set_dead_zone("AIM_UP".to_string(), 0.5);
    input_map.set_dead_zone("AIM_DOWN".to_string(), 0.5);
    input_map.set_dead_zone("AIM_LEFT".to_string(), 0.5);
    input_map.set_dead_zone("AIM_RIGHT".to_string(), 0.5);
}

/// This system prints 'A' key state
fn action_system(input_map: Res<InputMap>, mut app_exit_events: ResMut<Events<AppExit>>) {
    if input_map.is_action_in_progress("JUMP".to_string()) {
        println!("Jumping...");
    }

    if input_map.is_action_in_progress("SHOOT".to_string()) {
        println!("Bang");
    }

    if input_map.is_action_in_progress("AIM_UP".to_string()) {
        println!(
            "AIM_UP... [ strength: {}] ",
            input_map.get_action_strength("AIM_UP".to_string())
        );
    }

    if input_map.is_action_in_progress("AIM_DOWN".to_string()) {
        println!(
            "AIM_DOWN... [ strength: {}] ",
            input_map.get_action_strength("AIM_DOWN".to_string())
        );
    }

    if input_map.is_action_in_progress("AIM_LEFT".to_string()) {
        println!(
            "AIM_LEFT... [ strength: {}] ",
            input_map.get_action_strength("AIM_LEFT".to_string())
        );
    }

    if input_map.is_action_in_progress("AIM_RIGHT".to_string()) {
        println!(
            "AIM_RIGHT... [ strength: {}] ",
            input_map.get_action_strength("AIM_RIGHT".to_string())
        );
    }

    if input_map.is_action_in_progress("QUIT_APP".to_string()) {
        println!("Quiting...");
        app_exit_events.send(AppExit);
    }
}
