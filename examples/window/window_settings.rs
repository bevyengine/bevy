//! Illustrates how to change window settings and shows how to affect
//! the mouse pointer in various ways.

use bevy::{
    ecs::system::Command,
    prelude::*,
    window::{PresentMode, PrimaryWindow, WindowCursor, WindowTitle},
};

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "I am a window!".to_string(),
            width: 500.,
            height: 300.,
            present_mode: PresentMode::Fifo,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_system(change_title)
        .add_system(toggle_cursor)
        .add_system(cycle_cursor_icon)
        .run();
}

/// This system will then change the title during execution
fn change_title(mut commands: Commands, primary_window: Res<PrimaryWindow>, time: Res<Time>) {
    let mut window_commands = commands.window(primary_window.window.unwrap());
    window_commands.set_title(format!(
        "Seconds since startup: {}",
        time.seconds_since_startup().round()
    ));
}

/// This system toggles the cursor's visibility when the space bar is pressed
fn toggle_cursor(
    mut commands: Commands,
    primary_window: Res<PrimaryWindow>,
    window_q: Query<&WindowCursor, With<Window>>,
    input: Res<Input<KeyCode>>,
) {
    let primary_window_id = primary_window.window.unwrap();
    let mut window_commands = commands.window(primary_window_id);
    if input.just_pressed(KeyCode::Space) {
        let cursor = window_q.get(primary_window_id).unwrap(); // TODO: Is unwrap ok for these?
        window_commands.set_cursor_lock_mode(!cursor.cursor_locked());
        window_commands.set_cursor_visibility(!cursor.cursor_visible());
    }
}

/// This system cycles the cursor's icon through a small set of icons when clicking
fn cycle_cursor_icon(
    mut commands: Commands,
    primary_window: Res<PrimaryWindow>,
    input: Res<Input<MouseButton>>,
    mut index: Local<usize>,
) {
    const ICONS: &[CursorIcon] = &[
        CursorIcon::Default,
        CursorIcon::Hand,
        CursorIcon::Wait,
        CursorIcon::Text,
        CursorIcon::Copy,
    ];

    let primary_window_id = primary_window.window.unwrap();
    let mut window_commands = commands.window(primary_window_id);
    if input.just_pressed(MouseButton::Left) {
        *index = (*index + 1) % ICONS.len();
        window_commands.set_cursor_icon(ICONS[*index]);
    } else if input.just_pressed(MouseButton::Right) {
        *index = if *index == 0 {
            ICONS.len() - 1
        } else {
            *index - 1
        };
        window_commands.set_cursor_icon(ICONS[*index]);
    }
}
