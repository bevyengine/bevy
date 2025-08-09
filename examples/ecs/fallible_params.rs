//! This example demonstrates how fallible parameters can prevent their systems
//! from running if their acquiry conditions aren't met.
//!
//! Fallible system parameters include:
//! - [`Res<R>`], [`ResMut<R>`] - Resource has to exist, and the [`World::get_default_error_handler`] will be called if it doesn't.
//! - [`Single<D, F>`] - There must be exactly one matching entity, but the system will be silently skipped otherwise.
//! - [`Option<Single<D, F>>`] - There must be zero or one matching entity. The system will be silently skipped if there are more.
//! - [`Populated<D, F>`] - There must be at least one matching entity, but the system will be silently skipped otherwise.
//!
//! Other system parameters, such as [`Query`], will never fail validation: returning a query with no matching entities is valid.
//!
//! The result of failed system parameter validation is determined by the [`SystemParamValidationError`] returned
//! by [`SystemParam::validate_param`] for each system parameter.
//! Each system will pass if all of its parameters are valid, or else return [`SystemParamValidationError`] for the first failing parameter.
//!
//! To learn more about setting the fallback behavior for [`SystemParamValidationError`] failures,
//! please see the `error_handling.rs` example.
//!
//! [`SystemParamValidationError`]: bevy::ecs::system::SystemParamValidationError
//! [`SystemParam::validate_param`]: bevy::ecs::system::SystemParam::validate_param
//! [`default_error_handler`]: bevy::ecs::error::default_error_handler

use bevy::ecs::error::warn;
use bevy::prelude::*;
use rand::Rng;

fn main() {
    println!();
    println!("Press 'A' to add enemy ships and 'R' to remove them.");
    println!("Player ship will wait for enemy ships and track one if it exists,");
    println!("but will stop tracking if there are more than one.");
    println!();

    App::new()
        // By default, if a parameter fail to be fetched,
        // `World::get_default_error_handler` will be used to handle the error,
        // which by default is set to panic.
        .set_error_handler(warn)
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (user_input, move_targets, track_targets).chain())
        // This system will always fail validation, because we never create an entity with both `Player` and `Enemy` components.
        .add_systems(Update, do_nothing_fail_validation)
        .run();
}

/// Enemy component stores data for movement in a circle.
#[derive(Component, Default)]
struct Enemy {
    origin: Vec2,
    radius: f32,
    rotation: f32,
    rotation_speed: f32,
}

/// Player component stores data for going after enemies.
#[derive(Component, Default)]
struct Player {
    speed: f32,
    rotation_speed: f32,
    min_follow_radius: f32,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn 2D camera.
    commands.spawn(Camera2d);

    // Spawn player.
    let texture = asset_server.load("textures/simplespace/ship_C.png");
    commands.spawn((
        Player {
            speed: 100.0,
            rotation_speed: 2.0,
            min_follow_radius: 50.0,
        },
        Sprite {
            image: texture,
            color: bevy::color::palettes::tailwind::BLUE_800.into(),
            ..Default::default()
        },
        Transform::from_translation(Vec3::ZERO),
    ));
}

/// System that reads user input.
/// If user presses 'A' we spawn a new random enemy.
/// If user presses 'R' we remove a random enemy (if any exist).
fn user_input(
    mut commands: Commands,
    enemies: Query<Entity, With<Enemy>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    asset_server: Res<AssetServer>,
) {
    let mut rng = rand::rng();
    if keyboard_input.just_pressed(KeyCode::KeyA) {
        let texture = asset_server.load("textures/simplespace/enemy_A.png");
        commands.spawn((
            Enemy {
                origin: Vec2::new(
                    rng.random_range(-200.0..200.0),
                    rng.random_range(-200.0..200.0),
                ),
                radius: rng.random_range(50.0..150.0),
                rotation: rng.random_range(0.0..std::f32::consts::TAU),
                rotation_speed: rng.random_range(0.5..1.5),
            },
            Sprite {
                image: texture,
                color: bevy::color::palettes::tailwind::RED_800.into(),
                ..default()
            },
            Transform::from_translation(Vec3::ZERO),
        ));
    }

    if keyboard_input.just_pressed(KeyCode::KeyR)
        && let Some(entity) = enemies.iter().next()
    {
        commands.entity(entity).despawn();
    }
}

// System that moves the enemies in a circle.
// Only runs if there are enemies, due to the `Populated` parameter.
fn move_targets(mut enemies: Populated<(&mut Transform, &mut Enemy)>, time: Res<Time>) {
    for (mut transform, mut target) in &mut *enemies {
        target.rotation += target.rotation_speed * time.delta_secs();
        transform.rotation = Quat::from_rotation_z(target.rotation);
        let offset = transform.right() * target.radius;
        transform.translation = target.origin.extend(0.0) + offset;
    }
}

/// System that moves the player, causing them to track a single enemy.
/// If there is exactly one, player will track it.
/// Otherwise, the player will search for enemies.
fn track_targets(
    // `Single` ensures the system runs ONLY when exactly one matching entity exists.
    mut player: Single<(&mut Transform, &Player)>,
    // `Option<Single>` never prevents the system from running, but will be `None` if there is not exactly one matching entity.
    enemy: Option<Single<&Transform, (With<Enemy>, Without<Player>)>>,
    time: Res<Time>,
) {
    let (player_transform, player) = &mut *player;
    if let Some(enemy_transform) = enemy {
        // Enemy found, rotate and move towards it.
        let delta = enemy_transform.translation - player_transform.translation;
        let distance = delta.length();
        let front = delta / distance;
        let up = Vec3::Z;
        let side = front.cross(up);
        player_transform.rotation = Quat::from_mat3(&Mat3::from_cols(side, front, up));
        let max_step = distance - player.min_follow_radius;
        if 0.0 < max_step {
            let velocity = (player.speed * time.delta_secs()).min(max_step);
            player_transform.translation += front * velocity;
        }
    } else {
        // 0 or multiple enemies found, keep searching.
        player_transform.rotate_axis(Dir3::Z, player.rotation_speed * time.delta_secs());
    }
}

/// This system always fails param validation, because we never
/// create an entity with both [`Player`] and [`Enemy`] components.
fn do_nothing_fail_validation(_: Single<(), (With<Player>, With<Enemy>)>) {}
