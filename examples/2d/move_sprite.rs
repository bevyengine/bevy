//! Renders a 2D scene in which the *Bevy* logo is rendered as a sprite, moving up and down.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_moveables)
        .add_system(update_birds.before(update_moveables))
        .run();
}

/// A component for any sprite that might move.
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

/// Update everything that has both a velocity *and* a transform.
fn update_moveables(time: Res<Time>, mut query: Query<(&Velocity, &mut Transform)>) {
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
fn update_birds(mut query: Query<(&mut Velocity, &Transform), With<Bird>>) {
    for (mut v, trf) in &mut query {
        if trf.translation.y.abs() > 200. {
            v.0 *= -1.;
        }
    }
}
