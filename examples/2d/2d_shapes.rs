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
        mesh: meshes.add(Circle { radius: 50.0 }).into(),
        material: materials.add(Color::VIOLET),
        transform: Transform::from_translation(Vec3::new(-275.0, 0.0, 0.0)),
        ..default()
    });

    // Ellipse
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Ellipse::new(25.0, 50.0)).into(),
        material: materials.add(Color::TURQUOISE),
        transform: Transform::from_translation(Vec3::new(-150.0, 0.0, 0.0)),
        ..default()
    });

    // Capsule
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Capsule2d::new(25.0, 50.0)).into(),
        material: materials.add(Color::LIME_GREEN),
        transform: Transform::from_translation(Vec3::new(-50.0, 0.0, 0.0)),
        ..default()
    });

    // Rectangle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(Rectangle::new(50.0, 100.0)).into(),
        material: materials.add(Color::YELLOW),
        transform: Transform::from_translation(Vec3::new(50.0, 0.0, 0.0)),
        ..default()
    });

    // Hexagon
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(RegularPolygon::new(50.0, 6)).into(),
        material: materials.add(Color::ORANGE),
        transform: Transform::from_translation(Vec3::new(175.0, 0.0, 0.0)),
        ..default()
    });

    // Triangle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(Triangle2d::new(
                Vec2::Y * 50.0,
                Vec2::new(-50.0, -50.0),
                Vec2::new(50.0, -50.0),
            ))
            .into(),
        material: materials.add(Color::ORANGE_RED),
        transform: Transform::from_translation(Vec3::new(300.0, 0.0, 0.0)),
        ..default()
    });
}
