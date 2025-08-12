//! Shows how to display a window in transparent mode.
//!
//! This feature works as expected depending on the platform. Please check the
//! [documentation](https://docs.rs/bevy/latest/bevy/prelude/struct.Window.html#structfield.transparent)
//! for more details.

use bevy::camera::RenderTargetPremultipliedAlpha;
use bevy::prelude::*;
#[cfg(any(target_os = "macos", target_os = "linux"))]
use bevy::window::CompositeAlphaMode;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                // Setting `transparent` allows the `ClearColor`'s alpha value to take effect
                transparent: true,
                // Disabling window decorations to make it feel more like a widget than a window
                decorations: false,
                #[cfg(target_os = "macos")]
                composite_alpha_mode: CompositeAlphaMode::PostMultiplied,
                #[cfg(target_os = "linux")]
                composite_alpha_mode: CompositeAlphaMode::PreMultiplied,
                ..default()
            }),
            ..default()
        }))
        // ClearColor must have 0 alpha, otherwise some color will bleed through
        .insert_resource(ClearColor(Color::NONE))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    let camera = commands.spawn(Camera2d).id();

    if cfg!(target_os = "linux") {
        commands
            .entity(camera)
            .insert(RenderTargetPremultipliedAlpha);
    }

    for y in 0..10 {
        for x in 0..10 {
            let alpha = 0.1 + (x as f32 / 10.0) * 0.8; // Alpha from 0.1 to 0.9
            let brightness = y as f32 / 10.0; // Brightness from 0.0 to 1.0

            commands.spawn((
                Sprite {
                    color: Color::srgba(brightness, brightness, brightness, alpha),
                    custom_size: Some(Vec2::new(40.0, 40.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(
                    (x as f32 - 5.0) * 45.0,
                    (y as f32 - 5.0) * 45.0,
                    0.0,
                )),
            ));
        }
    }

    commands.spawn((
        Sprite {
            color: Color::srgba(1.0, 0.0, 0.0, 0.5), // Semi-transparent red
            custom_size: Some(Vec2::new(200.0, 200.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-100.0, 0.0, 1.0)),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgba(0.0, 0.0, 1.0, 0.5), // Semi-transparent blue
            custom_size: Some(Vec2::new(200.0, 200.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(100.0, 0.0, 1.0)),
    ));
}
