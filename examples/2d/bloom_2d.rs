//! Illustrates bloom post-processing in 2d.

use bevy::{core_pipeline::bloom::BloomSettings, prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::DARK_GRAY))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true, // 1. HDR is required for bloom
                ..default()
            },
            ..default()
        },
        BloomSettings::default(), // 2. Enable bloom for the camera
    ));

    // Sprite
    commands.spawn(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        sprite: Sprite {
            color: Color::hsl(0.0, 0.0, 2.5), // 3. Put something bright in a dark environment to see the effect
            custom_size: Some(Vec2::splat(160.0)),
            ..default()
        },
        ..default()
    });

    // Circle mesh
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes.add(shape::Circle::new(100.).into()).into(),
        // 3. Put something bright in a dark environment to see the effect
        material: materials.add(ColorMaterial::from(Color::rgb(1.5, 0.0, 1.5))),
        transform: Transform::from_translation(Vec3::new(-200., 0., 0.)),
        ..default()
    });

    // Hexagon mesh
    commands.spawn(MaterialMesh2dBundle {
        mesh: meshes
            .add(shape::RegularPolygon::new(100., 6).into())
            .into(),
        // 3. Put something bright in a dark environment to see the effect
        material: materials.add(ColorMaterial::from(Color::rgb(1.25, 1.88, 1.82))),
        transform: Transform::from_translation(Vec3::new(200., 0., 0.)),
        ..default()
    });
}
