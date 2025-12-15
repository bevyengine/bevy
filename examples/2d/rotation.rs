//! Demonstrates rotating entities in 2D using quaternions.

use bevy::{math::ops, prelude::*};

const BOUNDS: Vec2 = Vec2::new(1200.0, 640.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
            (
                player_movement_system,
                snap_to_player_system,
                rotate_to_player_system,
            ),
        )
        .run();
}

/// Player component
#[derive(Component)]
struct Player {
    /// Linear speed in meters per second
    movement_speed: f32,
    /// Rotation speed in radians per second
    rotation_speed: f32,
}

/// Snap to player ship behavior
#[derive(Component)]
struct SnapToPlayer;

/// Rotate to face player ship behavior
#[derive(Component)]
struct RotateToPlayer {
    /// Rotation speed in radians per second
    rotation_speed: f32,
}

/// Add the game's entities to our world and creates an orthographic camera for 2D rendering.
///
/// The Bevy coordinate system is the same for 2D and 3D, in terms of 2D this means that:
///
/// * `X` axis goes from left to right (`+X` points right)
/// * `Y` axis goes from bottom to top (`+Y` point up)
/// * `Z` axis goes from far to near (`+Z` points towards you, out of the screen)
///
/// The origin is at the center of the screen.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ship_handle = asset_server.load("textures/simplespace/ship_C.png");
    let enemy_a_handle = asset_server.load("textures/simplespace/enemy_A.png");
    let enemy_b_handle = asset_server.load("textures/simplespace/enemy_B.png");

    commands.spawn(Camera2d);

    // Create a minimal UI explaining how to interact with the example
    commands.spawn((
        Text::new("Up Arrow: Move Forward\nLeft / Right Arrow: Turn"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    let horizontal_margin = BOUNDS.x / 4.0;
    let vertical_margin = BOUNDS.y / 4.0;

    // Player controlled ship
    commands.spawn((
        Sprite::from_image(ship_handle),
        Player {
            movement_speed: 500.0,                  // Meters per second
            rotation_speed: f32::to_radians(360.0), // Degrees per second
        },
    ));

    // Enemy that snaps to face the player spawns on the bottom and left
    commands.spawn((
        Sprite::from_image(enemy_a_handle.clone()),
        Transform::from_xyz(0.0 - horizontal_margin, 0.0, 0.0),
        SnapToPlayer,
    ));
    commands.spawn((
        Sprite::from_image(enemy_a_handle),
        Transform::from_xyz(0.0, 0.0 - vertical_margin, 0.0),
        SnapToPlayer,
    ));

    // Enemy that rotates to face the player enemy spawns on the top and right
    commands.spawn((
        Sprite::from_image(enemy_b_handle.clone()),
        Transform::from_xyz(0.0 + horizontal_margin, 0.0, 0.0),
        RotateToPlayer {
            rotation_speed: f32::to_radians(45.0), // Degrees per second
        },
    ));
    commands.spawn((
        Sprite::from_image(enemy_b_handle),
        Transform::from_xyz(0.0, 0.0 + vertical_margin, 0.0),
        RotateToPlayer {
            rotation_speed: f32::to_radians(90.0), // Degrees per second
        },
    ));
}

/// Demonstrates applying rotation and movement based on keyboard input.
fn player_movement_system(
    time: Res<Time>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    query: Single<(&Player, &mut Transform)>,
) {
    let (ship, mut transform) = query.into_inner();

    let mut rotation_factor = 0.0;
    let mut movement_factor = 0.0;

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        rotation_factor += 1.0;
    }

    if keyboard_input.pressed(KeyCode::ArrowRight) {
        rotation_factor -= 1.0;
    }

    if keyboard_input.pressed(KeyCode::ArrowUp) {
        movement_factor += 1.0;
    }

    // Update the ship rotation around the Z axis (perpendicular to the 2D plane of the screen)
    transform.rotate_z(rotation_factor * ship.rotation_speed * time.delta_secs());

    // Get the ship's forward vector by applying the current rotation to the ships initial facing
    // vector
    let movement_direction = transform.rotation * Vec3::Y;
    // Get the distance the ship will move based on direction, the ship's movement speed and delta
    // time
    let movement_distance = movement_factor * ship.movement_speed * time.delta_secs();
    // Create the change in translation using the new movement direction and distance
    let translation_delta = movement_direction * movement_distance;
    // Update the ship translation with our new translation delta
    transform.translation += translation_delta;

    // Bound the ship within the invisible level bounds
    let extents = Vec3::from((BOUNDS / 2.0, 0.0));
    transform.translation = transform.translation.min(extents).max(-extents);
}

/// Demonstrates snapping the enemy ship to face the player ship immediately.
fn snap_to_player_system(
    mut query: Query<&mut Transform, (With<SnapToPlayer>, Without<Player>)>,
    player_transform: Single<&Transform, With<Player>>,
) {
    // Get the player translation in 2D
    let player_translation = player_transform.translation.xy();

    for mut enemy_transform in &mut query {
        // Get the vector from the enemy ship to the player ship in 2D and normalize it.
        let to_player = (player_translation - enemy_transform.translation.xy()).normalize();

        // Get the quaternion to rotate from the initial enemy facing direction to the direction
        // facing the player
        let rotate_to_player = Quat::from_rotation_arc(Vec3::Y, to_player.extend(0.));

        // Rotate the enemy to face the player
        enemy_transform.rotation = rotate_to_player;
    }
}

/// Demonstrates rotating an enemy ship to face the player ship at a given rotation speed.
///
/// This method uses the vector dot product to determine if the enemy is facing the player and
/// if not, which way to rotate to face the player. The dot product on two unit length vectors
/// will return a value between -1.0 and +1.0 which tells us the following about the two vectors:
///
/// * If the result is 1.0 the vectors are pointing in the same direction, the angle between them is
///   0 degrees.
/// * If the result is 0.0 the vectors are perpendicular, the angle between them is 90 degrees.
/// * If the result is -1.0 the vectors are parallel but pointing in opposite directions, the angle
///   between them is 180 degrees.
/// * If the result is positive the vectors are pointing in roughly the same direction, the angle
///   between them is greater than 0 and less than 90 degrees.
/// * If the result is negative the vectors are pointing in roughly opposite directions, the angle
///   between them is greater than 90 and less than 180 degrees.
///
/// It is possible to get the angle by taking the arc cosine (`acos`) of the dot product. It is
/// often unnecessary to do this though. Beware than `acos` will return `NaN` if the input is less
/// than -1.0 or greater than 1.0. This can happen even when working with unit vectors due to
/// floating point precision loss, so it pays to clamp your dot product value before calling
/// `acos`.
fn rotate_to_player_system(
    time: Res<Time>,
    mut query: Query<(&RotateToPlayer, &mut Transform), Without<Player>>,
    player_transform: Single<&Transform, With<Player>>,
) {
    // Get the player translation in 2D
    let player_translation = player_transform.translation.xy();

    for (config, mut enemy_transform) in &mut query {
        // Get the enemy ship forward vector in 2D (already unit length)
        let enemy_forward = (enemy_transform.rotation * Vec3::Y).xy();

        // Get the vector from the enemy ship to the player ship in 2D and normalize it.
        let to_player = (player_translation - enemy_transform.translation.xy()).normalize();

        // Get the dot product between the enemy forward vector and the direction to the player.
        let forward_dot_player = enemy_forward.dot(to_player);

        // If the dot product is approximately 1.0 then the enemy is already facing the player and
        // we can early out.
        if (forward_dot_player - 1.0).abs() < f32::EPSILON {
            continue;
        }

        // Get the right vector of the enemy ship in 2D (already unit length)
        let enemy_right = (enemy_transform.rotation * Vec3::X).xy();

        // Get the dot product of the enemy right vector and the direction to the player ship.
        // If the dot product is negative them we need to rotate counter clockwise, if it is
        // positive we need to rotate clockwise. Note that `copysign` will still return 1.0 if the
        // dot product is 0.0 (because the player is directly behind the enemy, so perpendicular
        // with the right vector).
        let right_dot_player = enemy_right.dot(to_player);

        // Determine the sign of rotation from the right dot player. We need to negate the sign
        // here as the 2D bevy co-ordinate system rotates around +Z, which is pointing out of the
        // screen. Due to the right hand rule, positive rotation around +Z is counter clockwise and
        // negative is clockwise.
        let rotation_sign = -f32::copysign(1.0, right_dot_player);

        // Limit rotation so we don't overshoot the target. We need to convert our dot product to
        // an angle here so we can get an angle of rotation to clamp against.
        let max_angle = ops::acos(forward_dot_player.clamp(-1.0, 1.0)); // Clamp acos for safety

        // Calculate angle of rotation with limit
        let rotation_angle =
            rotation_sign * (config.rotation_speed * time.delta_secs()).min(max_angle);

        // Rotate the enemy to face the player
        enemy_transform.rotate_z(rotation_angle);
    }
}
