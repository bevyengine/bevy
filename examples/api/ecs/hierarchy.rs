//! Demonstrates techniques for creating a hierarchy of parent and child entities.
//!
//! When [`DefaultPlugins`] are added to your app, systems are automatically added to propagate
//! [`Transform`] and [`Visibility`] from parents to children down the hierarchy,
//! resulting in a final [`GlobalTransform`] and [`InheritedVisibility`] component for each entity.

use std::f32::consts::*;

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    let texture = asset_server.load("branding/icon.png");

    // Spawn a root entity with no parent
    let parent = commands
        .spawn((
            Sprite::from_image(texture.clone()),
            Transform::from_scale(Vec3::splat(0.75)),
        ))
        // With that entity as a parent, run a lambda that spawns its children
        .with_children(|parent| {
            // parent is a ChildSpawnerCommands, which has a similar API to Commands
            parent.spawn((
                Transform::from_xyz(250.0, 0.0, 0.0).with_scale(Vec3::splat(0.75)),
                Sprite {
                    image: texture.clone(),
                    color: BLUE.into(),
                    ..default()
                },
            ));
        })
        // Store parent entity for next sections
        .id();

    // Another way is to use the add_child function to add children after the parent
    // entity has already been spawned.
    let child = commands
        .spawn((
            Sprite {
                image: texture,
                color: LIME.into(),
                ..default()
            },
            Transform::from_xyz(0.0, 250.0, 0.0).with_scale(Vec3::splat(0.75)),
        ))
        .id();

    // Add child to the parent.
    commands.entity(parent).add_child(child);
}

// A simple system to rotate the root entity, and rotate all its children separately
fn rotate(
    mut commands: Commands,
    time: Res<Time>,
    mut parents_query: Query<(Entity, &Children), With<Sprite>>,
    mut transform_query: Query<&mut Transform, With<Sprite>>,
) {
    for (parent, children) in &mut parents_query {
        if let Ok(mut transform) = transform_query.get_mut(parent) {
            transform.rotate_z(-PI / 2. * time.delta_secs());
        }

        // To iterate through the entities children, just treat the Children component as a Vec
        // Alternatively, you could query entities that have a ChildOf component
        for child in children {
            if let Ok(mut transform) = transform_query.get_mut(*child) {
                transform.rotate_z(PI * time.delta_secs());
            }
        }

        // To demonstrate removing children, we'll remove a child after a couple of seconds.
        if time.elapsed_secs() >= 2.0 && children.len() == 2 {
            let child = children.last().unwrap();
            commands.entity(*child).despawn();
        }

        if time.elapsed_secs() >= 4.0 {
            // This will remove the entity from its parent's list of children, as well as despawn
            // any children the entity has.
            commands.entity(parent).despawn();
        }
    }
}
