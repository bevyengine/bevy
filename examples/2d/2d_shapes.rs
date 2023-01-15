//! Shows how to render simple primitive shapes with a single color.

use bevy::{prelude::*, render::picking::Picking, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn((Camera2dBundle::default(), Picking::default()));

    // Circle
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(50.).into()).into(),
        material: materials.add(ColorMaterial::from(Color::PURPLE)),
        transform: Transform::from_translation(Vec3::new(-100., 0., 0.)),
        ..default()
    });

    // Rectangle
    let sprite_id = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.75),
                custom_size: Some(Vec2::new(50.0, 100.0)),
                ..default()
            },
            ..default()
        })
        .id();
    dbg!(sprite_id);

    // Rectangle
    let sprite_id = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.25, 0.35),
                custom_size: Some(Vec2::new(50.0, 100.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(200., 0., 0.)),
            ..default()
        })
        .id();
    dbg!(sprite_id);

    // Rectangle
    let sprite_id = commands
        .spawn(SpriteBundle {
            sprite: Sprite {
                color: Color::rgb(0.25, 0.85, 0.35),
                custom_size: Some(Vec2::new(50.0, 100.0)),
                ..default()
            },
            transform: Transform::from_translation(Vec3::new(-200., 0., 0.)),
            ..default()
        })
        .id();
    dbg!(sprite_id);

    // Hexagon
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::RegularPolygon::new(50., 6).into()).into(),
        material: materials.add(ColorMaterial::from(Color::TURQUOISE)),
        transform: Transform::from_translation(Vec3::new(100., 0., 0.)),
        ..default()
    });
}
