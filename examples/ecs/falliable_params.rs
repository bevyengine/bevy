//! This example demonstrates how falliable parameters can prevent systems from running
//! if their acquiry conditions aren't met.
//!
//! Falliable parameters include:
//! - [`Res<R>`], [`ResMut<R>`] - If resource doesn't exist.
//! - [`QuerySingle<D, F>`], [`QuerySingleMut<D, F>`] - If there is no or more than one entities matching.
//! - [`Option<QuerySingle<D, F>>`], [`Option<QuerySingleMut<D, F>>`] - If there are more than one entities matching.

use std::ops::DerefMut;

use bevy::{
    ecs::system::{QuerySingle, QuerySingleMut},
    prelude::*,
};
use rand::Rng;

fn main() {
    println!();

    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (user_input, move_targets, move_pointer).chain())
        .run();
}

#[derive(Component, Default)]
struct Enemy {
    origin: Vec2,
    radius: f32,
    rotation: f32,
    rotation_speed: f32,
}

#[derive(Component, Default)]
struct Player {
    speed: f32,
    rotation_speed: f32,
    stay_away: f32,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let texture = asset_server.load("textures/simplespace/ship_C.png");
    commands.spawn((
        Player {
            speed: 100.0,
            rotation_speed: 2.0,
            stay_away: 50.0,
        },
        SpriteBundle {
            transform: Transform::from_translation(Vec3::ZERO),
            texture,
            ..default()
        },
    ));
}

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
            SpriteBundle {
                sprite: Sprite {
                    color: bevy::color::palettes::basic::RED.into(),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::ZERO),
                texture,
                ..default()
            },
        ));
    }

    if keyboard_input.just_pressed(KeyCode::KeyR) {
        if let Some(entity) = enemies.iter().next() {
            commands.entity(entity).despawn();
        }
    }
}

// TODO: Use [`NonEmptyQuery`]
fn move_targets(mut enemies: Query<(&mut Transform, &mut Enemy)>, time: Res<Time>) {
    for (mut transform, mut target) in &mut enemies {
        target.rotation += target.rotation_speed * time.delta_seconds();
        transform.rotation = Quat::from_rotation_z(target.rotation);
        let offset = transform.right() * target.radius;
        transform.translation = target.origin.extend(0.0) + offset;
    }
}

/// Constantly rotates the pointer if there are no targets.
/// Points towards target if exactly one exists.
/// Does nothing (system doesn't run) when there are more than one entity.
fn move_pointer(
    mut player: QuerySingleMut<(&mut Transform, &Player)>,
    enemy: Option<QuerySingle<&Transform, (With<Enemy>, Without<Player>)>>,
    time: Res<Time>,
) {
    let (ref mut player_transform, ref player) = player.deref_mut();
    if let Some(enemy_transform) = enemy {
        let delta = enemy_transform.translation - player_transform.translation;
        let distance = delta.length();
        let front = delta / distance;
        let up = Vec3::Z;
        let side = front.cross(up);
        player_transform.rotation = Quat::from_mat3(&Mat3::from_cols(side, front, up));

        let come_closer = distance - player.stay_away;
        if 0.0 < come_closer {
            let velocity = (player.speed * time.delta_seconds()).min(come_closer);
            player_transform.translation += front * velocity;
        }
    } else {
        player_transform.rotate_axis(Dir3::Z, player.rotation_speed * time.delta_seconds());
    }
}
