use bevy::prelude::*;
use bevy_app::AppExit;
use bevy_prototype_input_map::inputmap::InputMap;
use bevy_prototype_input_map::keyboard::KeyboardMap;
use bevy_prototype_input_map::mouse::MouseMap;

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
    mut mouse_map: ResMut<MouseMap>
)
{
    // keyboard
    key_map.BindKeyboardPressed(KeyCode::Up, "JUMP".to_string());
    key_map.BindKeyboardPressed(KeyCode::Space, "JUMP".to_string());

    key_map.BindKeyboardPressed(KeyCode::Return, "SHOOT".to_string());

    key_map.BindKeyboardPressed(KeyCode::Escape, "QUIT_APP".to_string());

    // mouse
    mouse_map.BindMousePressed(MouseButton::Left, "SHOOT".to_string());
}

/// This system prints 'A' key state
fn action_system(
    input_map: Res<InputMap>,
    mut app_exit_events: ResMut<Events<AppExit>>
    ) {
        if input_map.IsActionPressed("JUMP".to_string())
        {
            println!("Jumping...");
        }

        if input_map.IsActionPressed("SHOOT".to_string())
        {
            println!("Bang");
        }

        if input_map.IsActionPressed("QUIT_APP".to_string())
        {
            println!("Quiting...");
            app_exit_events.send(AppExit);
        }
}
