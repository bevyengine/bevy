//! Demonstrates rotating sprites to face the cursor.

use bevy::prelude::*;
use std::f32::consts::FRAC_PI_2;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, player_movement_system)
        .run();
}

/// Player component
#[derive(Component)]
struct Player;

/// Add the game's entities to our world and create an orthographic camera for 2D rendering.
///
/// The Bevy coordinate system is the same for 2D and 3D, in terms of 2D this means that:
///
/// * `X` axis goes from left to right (`+X` points right)
/// * `Y` axis goes from bottom to top (`+Y` point up)
/// * `Z` axis goes from far to near (`+Z` points towards you, out of the screen)
///
/// The world origin in this case is at the center of the screen, but the camera could
/// move in which case the world origin would not be the center of the screen
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ship_handle = asset_server.load("textures/simplespace/ship_C.png");

    commands.spawn(Camera2d);

    // Player controlled ship
    commands.spawn((Sprite::from_image(ship_handle), Player));
}

/// Demonstrates applying rotation and movement based on keyboard input.
fn player_movement_system(
    mut player: Single<&mut Transform, With<Player>>,
    camera_query: Single<(&Camera, &GlobalTransform)>,
    window: Single<&Window>,
) {
    let (camera, camera_transform) = *camera_query;

    if let Some(cursor_position) = window.cursor_position()
        // Calculate a world position based on the cursor's position.
        && let Ok(cursor_world_pos) = camera.viewport_to_world_2d(camera_transform, cursor_position)
    {
        // The angle an entity needs to rotate to face a point is defined
        // by the vector between the two points (Vec2 - Vec2), which we can then
        // turn into radians using to_angle.
        //
        // FRAC_PI_2 is because our sprite's ship is facing "up" so we rotate it an additional 90 degrees
        // so that it faces the cursor.
        player.rotation = Quat::from_rotation_z(
            (cursor_world_pos - player.translation.xy()).to_angle() - FRAC_PI_2,
        );
    }
}
