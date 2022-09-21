//! Illustrates how to change window settings and shows how to affect
//! the mouse pointer in various ways.

use bevy::{prelude::*, window::PresentMode};
use bevy_internal::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "I am a window!".to_string(),
            width: 500.,
            height: 300.,
            present_mode: PresentMode::AutoVsync,
            ..default()
        })
        .insert_resource(VSync(true))
        .add_plugins(DefaultPlugins)
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_system(change_title)
        .add_system(toggle_cursor)
        .add_system(toggle_vsync)
        .add_system(cycle_cursor_icon)
        .run();
}

#[derive(Resource)]
pub struct VSync(pub bool);

/// This system toggles the vsync mode when pressing the button V.
/// You'll see fps increase displayed in the console.
fn toggle_vsync(
    input: Res<Input<KeyCode>>,
    mut vsync: ResMut<VSync>,
    mut windows: ResMut<Windows>,
) {
    if input.just_pressed(KeyCode::V) {
        vsync.0 = !vsync.0;
        if vsync.0 {
            windows
                .get_primary_mut()
                .unwrap()
                .set_present_mode(PresentMode::AutoVsync);
        } else {
            windows
                .get_primary_mut()
                .unwrap()
                .set_present_mode(PresentMode::AutoNoVsync);
        }
        info!(
            "PRESENT_MODE: {:?}",
            windows.get_primary().unwrap().present_mode()
        );
    }
}

/// This system will then change the title during execution
fn change_title(time: Res<Time>, mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    window.set_title(format!(
        "Seconds since startup: {}",
        time.seconds_since_startup().round()
    ));
}

/// This system toggles the cursor's visibility when the space bar is pressed
fn toggle_cursor(input: Res<Input<KeyCode>>, mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    if input.just_pressed(KeyCode::Space) {
        window.set_cursor_lock_mode(!window.cursor_locked());
        window.set_cursor_visibility(!window.cursor_visible());
    }
}

/// This system cycles the cursor's icon through a small set of icons when clicking
fn cycle_cursor_icon(
    input: Res<Input<MouseButton>>,
    mut windows: ResMut<Windows>,
    mut index: Local<usize>,
) {
    const ICONS: &[CursorIcon] = &[
        CursorIcon::Default,
        CursorIcon::Hand,
        CursorIcon::Wait,
        CursorIcon::Text,
        CursorIcon::Copy,
    ];
    let window = windows.primary_mut();
    if input.just_pressed(MouseButton::Left) {
        *index = (*index + 1) % ICONS.len();
        window.set_cursor_icon(ICONS[*index]);
    } else if input.just_pressed(MouseButton::Right) {
        *index = if *index == 0 {
            ICONS.len() - 1
        } else {
            *index - 1
        };
        window.set_cursor_icon(ICONS[*index]);
    }
}
