//! This example illustrates how have a mouse's clicks/wheel/movement etc fall through the spawned transparent window to a window below.
//! If you build this, and hit 'P' it should toggle on/off the mouse's passthrough.

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::NONE)) // Use a transparent window, to make effects obvious.
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Set the window's parameters, note we're setting the window to always be on top.
                transparent: true,
                decorations: true,
                always_on_top: true,
                ..default()
            }),
            ..default()
        }))
        .add_startup_system(setup)
        .add_system(toggle_mouse_passthrough) // This allows us to hit 'P' to toggle on/off the mouse's passthrough
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // UI camera
    commands.spawn(Camera2dBundle::default());
    // Text with one section
    commands.spawn((
        // Create a TextBundle that has a Text with a single section.
        TextBundle::from_section(
            // Accepts a `String` or any type that converts into a `String`, such as `&str`
            "Hit 'P' then scroll/click around!",
            TextStyle {
                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                font_size: 100.0, // Nice and big so you can see it!
                color: Color::WHITE,
            },
        )
        // Set the style of the TextBundle itself.
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                bottom: Val::Px(5.),
                right: Val::Px(10.),
                ..default()
            },
            ..default()
        }),
    ));
}
// A simple system to handle some keyboard input and toggle on/off the hittest.
fn toggle_mouse_passthrough(keyboard_input: Res<Input<KeyCode>>, mut windows: Query<&mut Window>) {
    if keyboard_input.just_pressed(KeyCode::P) {
        let mut window = windows.single_mut();
        window.cursor.hit_test = !window.cursor.hit_test;
    }
}
