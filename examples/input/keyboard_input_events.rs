use bevy::{input::{keyboard::KeyboardInput, ElementState}, prelude::*};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_system(print_keyboard_event_system.system())
        .run();
}

/// This system prints out all keyboard events as they come in
fn print_keyboard_event_system(mut keyboard_input_events: EventReader<KeyboardInput>) {
    for event in keyboard_input_events.iter() {
        let scan_code: u32 = event.scan_code;
        let key_code: Option<KeyCode> = event.key_code;
        let state: ElementState = event.state;
        info!("Scan code: {}", scan_code);
        info!("Key code: {:?}", key_code);
        match state {
            ElementState::Pressed => {
                info!("Pressed")
            }
            ElementState::Released => {
                info!("Released")
            }
        }
    }
}
