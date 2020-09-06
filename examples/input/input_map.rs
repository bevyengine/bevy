use bevy::prelude::*;
use bevy_prototype_input_map::keyboard::KeyboardMap;
use bevy_prototype_input_map::inputmap::InputMap;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(action_system.system())
        .run();
}

fn setup(
    mut key_map: ResMut<KeyboardMap>
)
{
    key_map.BindKeyboardPressed(KeyCode::Space, "JUMP".to_string(), 0.25);
}

/// This system prints 'A' key state
fn action_system(
    input_map: Res<InputMap>
    ) {
        if input_map.IsActionPressed("JUMP".to_string())
        {
            println!("Jumping...");
        }
        else
        {
            println!("Not Jumping...");
        }
}
