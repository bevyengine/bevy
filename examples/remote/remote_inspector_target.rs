//! Example target application for the remote inspector
//!
//! This application demonstrates a remote-inspectable Bevy app with bevy_remote enabled.
//! The inspector can connect to this app and view entities/components in real-time.
//!
//! ## Usage
//!
//! 1. Run this target application first:
//!    ```
//!    cargo run --example remote_inspector_target
//!    ```
//!
//! 2. In another terminal, run the inspector:
//!    ```  
//!    cargo run --example inspector_minimal
//!    ```
//!
//! The inspector will automatically connect to http://localhost:15702 and display
//! all entities and their components with live updates as values change.

use bevy::prelude::*;

#[derive(Component)]
struct Player {
    speed: f32,
    health: i32,
}

#[derive(Component)]
struct Velocity {
    x: f32,
    y: f32,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            // Enable remote inspection
            bevy::remote::RemotePlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_entities, rotate_entities))
        .run();
}

fn setup(mut commands: Commands) {
    info!("Setting up target application with bevy_remote enabled");
    info!("Remote inspector can connect to http://localhost:15702");

    // Spawn a camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("Main Camera"),
    ));

    // Spawn some test entities with various components
    commands.spawn((
        Transform::from_xyz(0.0, 0.0, 0.0),
        Visibility::Inherited,
        Player {
            speed: 5.0,
            health: 100,
        },
        Velocity { x: 1.0, y: 0.5 },
        Name::new("Player"),
    ));

    commands.spawn((
        Transform::from_xyz(2.0, 1.0, 0.0),
        Visibility::Inherited,
        Player {
            speed: 3.0,
            health: 75,
        },
        Name::new("Enemy 1"),
    ));

    commands.spawn((
        Transform::from_xyz(-2.0, -1.0, 0.0),
        Visibility::Hidden,
        Player {
            speed: 7.0,
            health: 50,
        },
        Velocity { x: -0.5, y: 1.0 },
        Name::new("Enemy 2"),
    ));

    // Spawn some unnamed entities
    commands.spawn((Transform::from_xyz(1.0, 2.0, 1.0), Visibility::Inherited));

    commands.spawn((
        Transform::from_xyz(-1.0, -2.0, -1.0),
        Player {
            speed: 2.0,
            health: 25,
        },
    ));

    info!("Spawned {} entities for inspection", 6);
}

fn move_entities(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in query.iter_mut() {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();

        // Wrap around
        if transform.translation.x > 5.0 {
            transform.translation.x = -5.0;
        }
        if transform.translation.x < -5.0 {
            transform.translation.x = 5.0;
        }
        if transform.translation.y > 3.0 {
            transform.translation.y = -3.0;
        }
        if transform.translation.y < -3.0 {
            transform.translation.y = 3.0;
        }
    }
}

fn rotate_entities(mut query: Query<&mut Transform, With<Player>>, time: Res<Time>) {
    for mut transform in query.iter_mut() {
        transform.rotate_y(time.delta_secs() * 0.5);
    }
}
