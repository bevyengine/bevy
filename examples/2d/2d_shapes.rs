//! Shows how to render simple primitive shapes with a single color.

use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dBundle::default());

    // Circle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(primitives::Circle { radius: 50.0 }.into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        transform: Transform::from_translation(Vec3::new(-150.0, 0.0, 0.0)),
        ..default()
    });

    // Ellipse
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(primitives::Ellipse::new(50.0, 100.0).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::rgb(0.25, 0.25, 0.75))),
        transform: Transform::from_translation(Vec3::new(-50.0, 0.0, 0.0)),
        ..default()
    });

    // Rectangle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(primitives::Rectangle::new(50.0, 100.0).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::LIME_GREEN)),
        transform: Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
        ..default()
    });

    // Hexagon
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(primitives::RegularPolygon::new(50.0, 6).into())
            .into(),
        material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
        transform: Transform::from_translation(Vec3::new(150.0, 0.0, 0.0)),
        ..default()
    });
}
