//! Displays a single [`Sprite`], created from an image.

use bevy::prelude::*;
use bevy::window::{WindowRef, WindowResolution};
use bevy_render::camera::RenderTarget;
use bevy_render::view::RenderLayers;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resolution: WindowResolution::new(1000., 1000.).with_scale_factor_override(1.),
                title: "Window 1".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // The primary window is the default camera target and 0 is the default render layer, there's no need to set these normally
    // the explicitness is just to make this example as clear as possible.

    commands.spawn((
        Camera2d::default(),
        Camera {
            target: RenderTarget::Window(WindowRef::Primary),
            ..default()
        },
        RenderLayers::layer(0),
    ));

    // Spawn a second window
    let window_2 = commands
        .spawn(Window {
            title: "Window 2".to_owned(),
            resolution: WindowResolution::new(1000., 1000.).with_scale_factor_override(1.),
            ..default()
        })
        .id();

    commands.spawn((
        Camera2d::default(),
        Camera {
            target: RenderTarget::Window(WindowRef::Entity(window_2)),
            ..default()
        },
        RenderLayers::layer(1),
    ));

    // Text drawn to window 1
    commands.spawn((Text2d::new("1"), RenderLayers::layer(0)));

    // Text drawn to window 2
    commands.spawn((Text2d::new("2"), RenderLayers::layer(1)));

    commands.spawn((
        Text2d::new("3"),
        Transform::from_translation(-25. * Vec3::Y),
        RenderLayers::from_layers(&[0, 1]),
    ));
}
