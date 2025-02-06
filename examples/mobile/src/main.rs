use bevy::prelude::*;
use bevy::window::{PrimaryWindow, WindowResolution};

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: "../../assets".to_string(),
                ..default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(360.0, 640.0),
                    ..default()
                }),
                ..default()
            })
            .add(bevy_mobile_example::MainPlugin),
    )
    .add_systems(Update, toggle_window_orientation)
    .run();
}

fn toggle_window_orientation(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keyboard_input.pressed(KeyCode::KeyL) {
        let mut window = windows.single_mut();
        window.resolution = WindowResolution::new(640.0, 360.0);
    }
    if keyboard_input.pressed(KeyCode::KeyP) {
        let mut window = windows.single_mut();
        window.resolution = WindowResolution::new(360.0, 640.0);
    }
}
