//! Example for `PanCam`, demonstrating basic camera controls such as panning and zooming.
//!
//! This example shows how to use the `PanCam` controller on a 2D camera in Bevy. The camera
//! can be panned with keyboard inputs (arrow keys or WASD) and zoomed in/out using the mouse
//! wheel or the +/- keys.
//!
//! Controls:
//! - Arrow keys (or WASD) to pan the camera.
//! - Mouse scroll wheel or +/- to zoom in/out.
//!
//! Configuration Options:
//! - `zoom_factor`: The initial zoom factor (default is 1.0).
//! - `zoom_speed`: The sensitivity of zooming (default is 0.1).
//! - `min_zoom` and `max_zoom`: Set the bounds for zooming (default is 0.1 and 5.0).
//! - `pan_speed`: The speed at which the camera moves (default is 500.0).

use bevy::camera_controller::pan_cam::{PanCam, PanCamPlugin};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(PanCamPlugin) // Adds the PanCam plugin to enable camera panning and zooming controls.
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn a 2D Camera with custom PanCam settings
    commands.spawn((
        Camera2d,
        PanCam {
            enable: true,                  // Enable the PanCam controller
            zoom_factor: 1.0,              // Initial zoom level (100%)
            zoom_speed: 0.2,               // Zoom sensitivity
            min_zoom: 0.5,                 // Minimum zoom level (50%)
            max_zoom: 5.0,                 // Maximum zoom level (500%)
            pan_speed: 600.0,              // Camera pan speed
            key_up: KeyCode::KeyW,         // Move up with W key
            key_down: KeyCode::KeyS,       // Move down with S key
            key_left: KeyCode::KeyA,       // Move left with A key
            key_right: KeyCode::KeyD,      // Move right with D key
            key_zoom_in: KeyCode::Equal,   // Zoom in with '+' key
            key_zoom_out: KeyCode::Minus,  // Zoom out with '-' key
            key_rotate_ccw: KeyCode::KeyQ, // Rotate counter-clockwise with Q key
            key_rotate_cw: KeyCode::KeyE,  // Rotate clockwise with E key
            ..default()                    // Use default values for other fields
        },
    ));

    commands.spawn(Sprite::from_image(
        asset_server.load("branding/bevy_bird_dark.png"),
    ));
}
