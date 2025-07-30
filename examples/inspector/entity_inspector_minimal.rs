//! A minimal example showcasing the Entity Inspector plugin.
//!
//! This example creates a few entities with custom components to demonstrate
//! how the Entity Inspector can be used to inspect and debug your Bevy application.
//!
//! Press F12 to toggle the Entity Inspector window.

use bevy::prelude::*;
use bevy::dev_tools::entity_inspector::EntityInspectorPlugin;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Add the Entity Inspector plugin
        .add_plugins(EntityInspectorPlugin)
        // Register custom components for reflection (required for inspector to show component data)
        .register_type::<CustomData>()
        .register_type::<GameTimer>()
        .register_type::<Rotating>()
        .add_systems(Startup, setup)
        .add_systems(Update, (rotate_cube, update_timer))
        .run();
}

/// Custom component to demonstrate reflection
#[derive(Component, Reflect, Default)]
#[reflect(Component)]
struct CustomData {
    value: f32,
    name: String,
    active: bool,
}

/// Another custom component
#[derive(Component, Reflect)]
#[reflect(Component)]
struct GameTimer {
    elapsed: f32,
    max_time: f32,
}

/// Component to mark rotating objects
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Rotating {
    speed: f32,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 5.0),
        Name::new("Main Camera"),
    ));

    // Light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -std::f32::consts::PI / 4.)),
        Name::new("Directional Light"),
    ));

    // Rotating cube with custom components
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.6))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        CustomData {
            value: 42.0,
            name: "Rotating Cube".to_string(),
            active: true,
        },
        Rotating { speed: 1.0 },
        Name::new("Magic Cube"),
    ));

    // Static sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.8, 0.3))),
        Transform::from_xyz(2.0, 0.0, 0.0),
        CustomData {
            value: 100.0,
            name: "Static Sphere".to_string(),
            active: false,
        },
        Name::new("Green Sphere"),
    ));

    // Game timer entity (no mesh, just data)
    commands.spawn((
        GameTimer {
            elapsed: 0.0,
            max_time: 60.0,
        },
        Name::new("Game Timer"),
    ));

    // UI instructions
    commands.spawn((
        Text::new("Press F12 to open Entity Inspector\n\nClick on entities in the left pane to inspect their components.\nThe inspector shows all registered, reflectable components."),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        TextFont {
            font_size: 18.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Name::new("Instructions"),
    ));
}

/// Rotate cubes marked with the Rotating component
fn rotate_cube(
    mut query: Query<(&mut Transform, &Rotating)>,
    time: Res<Time>,
) {
    for (mut transform, rotating) in query.iter_mut() {
        transform.rotate_y(rotating.speed * time.delta_secs());
        transform.rotate_x(0.5 * rotating.speed * time.delta_secs());
    }
}

/// Update the game timer
fn update_timer(
    mut timer_query: Query<&mut GameTimer>,
    time: Res<Time>,
) {
    for mut timer in timer_query.iter_mut() {
        timer.elapsed += time.delta_secs();
        if timer.elapsed >= timer.max_time {
            timer.elapsed = 0.0;
            info!("Timer reset!");
        }
    }
}