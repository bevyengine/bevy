use bevy::prelude::*;
use bevy_app::AppExit;
use bevy_prototype_input_map::inputmap::InputMap;
use bevy_prototype_input_map::keyboard::KeyboardMap;
use bevy_prototype_input_map::mouse::MouseMap;
use bevy_prototype_input_map::axis::Axis;

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
    mut input_map: ResMut<InputMap>
)
{
    // keyboard
    key_map.BindKeyboardPressed(KeyCode::Space, "JUMP".to_string());
    key_map.BindKeyboardPressed(KeyCode::Return, "SHOOT".to_string());

    key_map.BindKeyboardPressed(KeyCode::Escape, "QUIT_APP".to_string());

    // mouse
    mouse_map.BindMousePressed(MouseButton::Left, "SHOOT".to_string());
    mouse_map.BindMousePressed(MouseButton::Right, "JUMP".to_string());

    mouse_map.BindMouseMove(Axis::Y_Negative, "AIM_UP".to_string());
    mouse_map.BindMouseMove(Axis::Y_Positive, "AIM_DOWN".to_string());
    mouse_map.BindMouseMove(Axis::X_Negative, "AIM_LEFT".to_string());
    mouse_map.BindMouseMove(Axis::X_Positive, "AIM_RIGHT".to_string());

    // input map
    // meaningful only for analog inputs like mouse move, joystick...etc
    input_map.SetDeadZone("AIM_UP".to_string(), 0.5);
    input_map.SetDeadZone("AIM_DOWN".to_string(), 0.5);
    input_map.SetDeadZone("AIM_LEFT".to_string(), 0.5);
    input_map.SetDeadZone("AIM_RIGHT".to_string(), 0.5);

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

        if input_map.IsActionPressed("AIM_UP".to_string())
        {
            println!("AIM_UP... [ strength: {}] ", input_map.GetActionStrength("AIM_UP".to_string()));
        }

        if input_map.IsActionPressed("AIM_DOWN".to_string())
        {
            println!("AIM_DOWN... [ strength: {}] ", input_map.GetActionStrength("AIM_DOWN".to_string()));
        }

        if input_map.IsActionPressed("AIM_LEFT".to_string())
        {
            println!("AIM_LEFT... [ strength: {}] ", input_map.GetActionStrength("AIM_LEFT".to_string()));
        }

        if input_map.IsActionPressed("AIM_RIGHT".to_string())
        {
            println!("AIM_RIGHT... [ strength: {}] ", input_map.GetActionStrength("AIM_RIGHT".to_string()));
        }

        if input_map.IsActionPressed("QUIT_APP".to_string())
        {
            println!("Quiting...");
            app_exit_events.send(AppExit);
        }
}
