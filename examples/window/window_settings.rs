use bevy::{
    input::keyboard::{modifiers::ALT, KeyModifiers},
    prelude::*,
    window::WindowMode,
};

/// This example illustrates how to customize the default window settings
fn main() {
    App::build()
        .add_resource(WindowDescriptor {
            title: "I am a window!".to_string(),
            width: 500,
            height: 300,
            vsync: true,
            resizable: false,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_system(change_title)
        .add_system(toggle_cursor)
        .add_system(toggle_fullscreen)
        .run();
}

/// This system will then change the title during execution
fn change_title(time: Res<Time>, mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    window.set_title(format!(
        "Seconds since startup: {}",
        time.seconds_since_startup().round()
    ));
}

/// This system toggles the cursor's visibility when the space bar is pressed
fn toggle_cursor(input: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    if input.just_pressed(KeyCode::Space) {
        window.set_cursor_lock_mode(!window.cursor_locked());
        window.set_cursor_visibility(!window.cursor_visible());
    }
}

fn toggle_fullscreen(
    key_codes: Res<Input<KeyCode>>,
    key_modifiers: Res<KeyModifiers>,
    mut windows: ResMut<Windows>,
) {
    if key_codes.just_pressed(KeyCode::Return) && *key_modifiers == ALT {
        let window = windows.get_primary_mut().unwrap();
        match window.mode() {
            WindowMode::BorderlessFullscreen => window.set_mode(WindowMode::Windowed),
            WindowMode::Windowed => window.set_mode(WindowMode::BorderlessFullscreen),
            _ => (),
        }
    }
}
