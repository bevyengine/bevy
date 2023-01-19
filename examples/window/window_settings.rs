//! Illustrates how to change window settings and shows how to affect
//! the mouse pointer in various ways.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{CursorGrabMode, PresentMode},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "I am a window!".into(),
                resolution: (500., 300.).into(),
                present_mode: PresentMode::AutoVsync,
                // Tells wasm to resize the window according to the available canvas
                fit_canvas_to_parent: true,
                ..default()
            }),
            ..default()
        }))
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin)
        .add_system(change_title)
        .add_system(toggle_cursor)
        .add_system(toggle_vsync)
        .add_system(cycle_cursor_icon)
        .add_system(toggle_always_on_top)
        .run();
}

/// This system toggles the vsync mode when pressing the button V.
/// You'll see fps increase displayed in the console.
fn toggle_vsync(input: Res<Input<KeyCode>>, mut windows: Query<&mut Window>) {
    if input.just_pressed(KeyCode::V) {
        let mut window = windows.single_mut();

        window.present_mode = if matches!(window.present_mode, PresentMode::AutoVsync) {
            PresentMode::AutoNoVsync
        } else {
            PresentMode::AutoVsync
        };
        info!("PRESENT_MODE: {:?}", window.present_mode);
    }
}

/// This system toggles whether the window is always on top when pressing the T button
/// You'll notice it won't be covered by other windows.
///
/// This feature only works on some platforms. Please check the
/// [documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.WindowDescriptor.html#structfield.always_on_top)
/// for more details.
fn toggle_always_on_top(input: Res<Input<KeyCode>>, mut windows: Query<&mut Window>) {
    if input.just_pressed(KeyCode::T) {
        let mut window = windows.single_mut();

        window.always_on_top = !window.always_on_top;

        if window.always_on_top {
            info!("LOCKING WINDOW ON TOP");
        } else {
            info!("UNLOCKING WINDOW");
        }
    }
}

/// This system will then change the title during execution
fn change_title(mut windows: Query<&mut Window>, time: Res<Time>) {
    let mut window = windows.single_mut();
    window.title = format!(
        "Seconds since startup: {}",
        time.elapsed().as_secs_f32().round()
    );
}

fn toggle_cursor(mut windows: Query<&mut Window>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Space) {
        let mut window = windows.single_mut();

        window.cursor.visible = !window.cursor.visible;
        window.cursor.grab_mode = match window.cursor.grab_mode {
            CursorGrabMode::None => CursorGrabMode::Locked,
            CursorGrabMode::Locked | CursorGrabMode::Confined => CursorGrabMode::None,
        };
    }
}

/// This system cycles the cursor's icon through a small set of icons when clicking
fn cycle_cursor_icon(
    mut windows: Query<&mut Window>,
    input: Res<Input<MouseButton>>,
    mut index: Local<usize>,
) {
    let mut window = windows.single_mut();

    const ICONS: &[CursorIcon] = &[
        CursorIcon::Default,
        CursorIcon::Hand,
        CursorIcon::Wait,
        CursorIcon::Text,
        CursorIcon::Copy,
    ];

    if input.just_pressed(MouseButton::Left) {
        *index = (*index + 1) % ICONS.len();
    } else if input.just_pressed(MouseButton::Right) {
        *index = if *index == 0 {
            ICONS.len() - 1
        } else {
            *index - 1
        };
    }

    window.cursor.icon = ICONS[*index];
}
