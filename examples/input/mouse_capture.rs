use bevy::{input::mouse::MouseButtonInput, prelude::*, window::WindowId, winit::WinitWindows};

/// This example illustrates how to capture the mouse cursor (grab it and hide it).
/// Press left mouse button to capture the mouse cursor and right mouse button to release it.
fn main() {
    App::build()
        .add_default_plugins()
        .init_resource::<State>()
        .add_system(mouse_capture_system.system())
        .run();
}

#[derive(Default)]
struct State {
    mouse_button_event_reader: EventReader<MouseButtonInput>,
}

fn mouse_capture_system(
    mut state: ResMut<State>,
    mouse_button_events: Res<Events<MouseButtonInput>>,
    windows: Res<WinitWindows>,
) {
    if let Some(event) = state.mouse_button_event_reader.latest(&mouse_button_events) {
        let window = windows.get_window(WindowId::primary()).unwrap();
        match event.button {
            MouseButton::Left => {
                window.set_cursor_grab(true).unwrap();
                window.set_cursor_visible(false);
            }
            MouseButton::Right => {
                window.set_cursor_grab(false).unwrap();
                window.set_cursor_visible(true);
            }
            _ => (),
        }
    }
}
