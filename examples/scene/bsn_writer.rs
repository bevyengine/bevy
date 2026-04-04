//! Demonstrates serializing a Bevy scene to BSN text.
//!
//! The scene contains a floor with a cube on it, a light, and a camera.
//! Components include transforms, meshes, materials, and a custom enum
//! to show how enum variant fields are serialized.
//!
//! Press S to serialize with default config, or D to also skip camera components.
//! The serialized BSN text is printed to the console via `info!`.

use bevy::{
    prelude::*,
    scene2::dynamic_bsn_writer::{serialize_to_bsn, serialize_to_bsn_with_config, BsnWriterConfig},
};

/// Example enum component to demonstrate variant field serialization.
#[derive(Component, Reflect, Default, Debug, PartialEq)]
#[reflect(Component, Default, PartialEq)]
enum CollisionShape {
    #[default]
    None,
    Sphere {
        radius: f32,
    },
    Box {
        width: f32,
        height: f32,
        depth: f32,
    },
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .register_type::<CollisionShape>()
        .add_systems(Startup, setup)
        .add_systems(Update, write_on_keypress)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // UI elements intentionally have no Name, so they are excluded from serialization.
    // The writer only serializes named entities.
    commands.spawn((
        Camera2d,
        Camera {
            order: 1,
            ..default()
        },
    ));

    commands.spawn((
        Text::new("Press S to serialize scene\nPress D to serialize (skip camera components)"),
        TextFont {
            font_size: FontSize::Px(20.0),
            ..default()
        },
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));

    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        Transform::from_xyz(0.0, 5.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Light"),
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -1.0, 0.5, 0.0)),
    ));

    commands
        .spawn((
            Name::new("Floor"),
            Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(5.0)))),
            MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
            // Enum with struct variant fields
            CollisionShape::Box {
                width: 10.0,
                height: 0.1,
                depth: 10.0,
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Cube"),
                Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
                MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
                Transform::from_xyz(0.0, 0.5, 0.0),
                // Enum with struct variant fields
                CollisionShape::Sphere { radius: 0.5 },
            ));
        });
}

fn write_on_keypress(world: &World, input: Res<ButtonInput<KeyCode>>) {
    // Default config: skips internal Bevy runtime components
    if input.just_pressed(KeyCode::KeyS) {
        let bsn_text = serialize_to_bsn(world);
        info!("{bsn_text}");
    }

    // Custom config: also skip camera-related components
    if input.just_pressed(KeyCode::KeyD) {
        let config = BsnWriterConfig::default()
            .skip_prefix("bevy_camera::")
            .skip_prefix("bevy_core_pipeline::");

        let bsn_text = serialize_to_bsn_with_config(world, &config);
        info!("{bsn_text}");
    }
}
