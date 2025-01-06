//! This example demonstrates how fallible parameters can prevent their systems
//! from running if their acquiry conditions aren't met.
//!
//! Fallible parameters include:
//! - [`Res<R>`], [`ResMut<R>`] - Resource has to exist.
//! - [`Single<D, F>`] - There must be exactly one matching entity.
//! - [`Option<Single<D, F>>`] - There must be zero or one matching entity.
//! - [`Populated<D, F>`] - There must be at least one matching entity.

use bevy::prelude::*;
use rand::Rng;

fn main() {
    println!();
    println!("Press 'A' to add enemy ships and 'R' to remove them.");
    println!("Player ship will wait for enemy ships and track one if it exists,");
    println!("but will stop tracking if there are more than one.");
    println!();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // Default system policy is to panic if parameters fail to be fetched.
        // We overwrite that configuration, to either warn us once or never.
        // This is good for catching unexpected behavior without crashing the app,
        // but can lead to spam.
        .add_systems(
            Update,
            (
                user_input.param_warn_once(),
                move_targets.never_param_warn(),
                move_pointer.never_param_warn(),
            )
                .chain(),
        )
        .add_systems(Update, do_nothing_fail_validation.param_warn_once())
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
    let mut rng = rand::thread_rng();
    if keyboard_input.just_pressed(KeyCode::KeyA) {
        let texture = asset_server.load("textures/simplespace/enemy_A.png");
        commands.spawn((
            Enemy {
                origin: Vec2::new(rng.gen_range(-200.0..200.0), rng.gen_range(-200.0..200.0)),
                radius: rng.gen_range(50.0..150.0),
                rotation: rng.gen_range(0.0..std::f32::consts::TAU),
                rotation_speed: rng.gen_range(0.5..1.5),
            },
            Sprite {
                image: texture,
                color: bevy::color::palettes::tailwind::RED_800.into(),
                ..default()
            },
            Transform::from_translation(Vec3::ZERO),
        ));
    }

    if keyboard_input.just_pressed(KeyCode::KeyR) {
        if let Some(entity) = enemies.iter().next() {
            commands.entity(entity).despawn();
        }
    }
}

// System that moves the enemies in a circle.
// Only runs if there are enemies.
fn move_targets(mut enemies: Populated<(&mut Transform, &mut Enemy)>, time: Res<Time>) {
    for (mut transform, mut target) in &mut *enemies {
        target.rotation += target.rotation_speed * time.delta_secs();
        transform.rotation = Quat::from_rotation_z(target.rotation);
        let offset = transform.right() * target.radius;
        transform.translation = target.origin.extend(0.0) + offset;
    }
}

/// System that moves the player.
/// The player will search for enemies if there are none.
/// If there is one, player will track it.
/// If there are too many enemies, the player will cease all action (the system will not run).
fn move_pointer(
    // `Single` ensures the system runs ONLY when exactly one matching entity exists.
    mut player: Single<(&mut Transform, &Player)>,
    // `Option<Single>` ensures that the system runs ONLY when zero or one matching entity exists.
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
        // No enemy found, keep searching.
        player_transform.rotate_axis(Dir3::Z, player.rotation_speed * time.delta_secs());
    }
}

/// This system always fails param validation, because we never
/// create an entity with both [`Player`] and [`Enemy`] components.
fn do_nothing_fail_validation(_: Single<(), (With<Player>, With<Enemy>)>) {}
