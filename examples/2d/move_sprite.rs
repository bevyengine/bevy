//! Renders a 2D scene in which the *Bevy* logo is rendered as a sprite, moving up and down.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_bird_velocity)
        .add_system(apply_velocity.after(update_bird_velocity))
        .run();
}

/// A component for any moving entity.
#[derive(Component)]
struct Velocity(Vec2);

/// A marker component for our particular sprite.
#[derive(Component)]
struct Bird;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        Bird,
        SpriteBundle {
            texture: asset_server.load("branding/icon.png"),
            transform: Transform::from_xyz(100., 0., 0.),
            ..default()
        },
        Velocity(Vec2 { x: 0., y: 150. }),
    ));
}

/// Update all moving entities: those with a velocity and a transform.
fn apply_velocity(time: Res<Time>, mut query: Query<(&Velocity, &mut Transform)>) {
    let delta = time.delta_seconds();
    for (velocity, mut transform) in &mut query {
        transform.translation += velocity.0.extend(0.0) * delta;
    }
}

/// Update bird sprites according to our specific rules.
///
/// Many components are widely used and few are as ubiquitous as `Transform`;
/// component reuse is one of the core tenets of Entity Component Systems (ECS).
///
/// This system uses a filter – `With<Bird>` – to restrict updates to bird sprites, only.
fn update_bird_velocity(mut query: Query<(&mut Velocity, &Transform), With<Bird>>) {
    for (mut velocity, transform) in &mut query {
        if transform.translation.y.abs() > 200. {
            velocity.0 *= -1.;
        }
    }
}
