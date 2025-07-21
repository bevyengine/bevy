//! Showcase how to use the in-game inspector for debugging entities and components.

use bevy::{
    dev_tools::inspector::{InspectorConfig, InspectorPlugin, InspectorPosition},
    prelude::*,
};

// Example component for demonstration
#[derive(Component)]
struct Player {
    speed: f32,
    health: i32,
}

#[derive(Component)]
struct Position {
    x: f32,
    y: f32,
}

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            InspectorPlugin {
                config: InspectorConfig {
                    enabled: false, // Start hidden, toggle with F3
                    toggle_key: KeyCode::F3,
                    position: InspectorPosition::TopLeft,
                },
            },
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_player, update_positions))
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn a camera
    commands.spawn(Camera2d);

    // Spawn some example entities to inspect
    commands.spawn((
        Player {
            speed: 100.0,
            health: 100,
        },
        Position { x: 0.0, y: 0.0 },
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        Name::new("Player"),
    ));

    // Spawn some collectible items
    for i in 0..5 {
        commands.spawn((
            Position {
                x: (i as f32) * 50.0,
                y: 100.0,
            },
            Transform::from_translation(Vec3::new((i as f32) * 50.0, 100.0, 0.0)),
            Name::new(format!("Item {}", i + 1)),
        ));
    }

    // Spawn an enemy
    commands.spawn((
        Player {
            speed: 75.0,
            health: 50,
        },
        Position { x: 200.0, y: 200.0 },
        Transform::from_translation(Vec3::new(200.0, 200.0, 0.0)),
        Name::new("Enemy"),
    ));
}

fn move_player(
    mut query: Query<(&mut Position, &Player), With<Player>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    for (mut position, player) in query.iter_mut() {
        let mut direction = Vec2::ZERO;

        if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
            direction.x -= 1.0;
        }
        if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
            direction.x += 1.0;
        }
        if keyboard.pressed(KeyCode::ArrowUp) || keyboard.pressed(KeyCode::KeyW) {
            direction.y += 1.0;
        }
        if keyboard.pressed(KeyCode::ArrowDown) || keyboard.pressed(KeyCode::KeyS) {
            direction.y -= 1.0;
        }

        if direction.length() > 0.0 {
            direction = direction.normalize();
            position.x += direction.x * player.speed * time.delta_secs();
            position.y += direction.y * player.speed * time.delta_secs();
        }
    }
}

fn update_positions(mut query: Query<(&Position, &mut Transform), Changed<Position>>) {
    for (position, mut transform) in query.iter_mut() {
        transform.translation.x = position.x;
        transform.translation.y = position.y;
    }
}
