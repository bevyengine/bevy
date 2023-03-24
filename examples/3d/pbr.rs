//! This example shows how to configure Physically Based Rendering (PBR) parameters.

use bevy::{asset::LoadState, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, environment_map_load_finish)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // add entities to the world
    for y in -2..=2 {
        for x in -5..=5 {
            let x01 = (x + 5) as f32 / 10.0;
            let y01 = (y + 2) as f32 / 4.0;
            // sphere
            commands.spawn(PbrBundle {
                mesh: meshes.add(
                    Mesh::try_from(shape::Icosphere {
                        radius: 0.45,
                        subdivisions: 32,
                    })
                    .unwrap(),
                ),
                material: materials.add(StandardMaterial {
                    base_color: Color::hex("#ffd891").unwrap(),
                    // vary key PBR parameters on a grid of spheres to show the effect
                    metallic: y01,
                    perceptual_roughness: x01,
                    ..default()
                }),
                transform: Transform::from_xyz(x as f32, y as f32 + 0.5, 0.0),
                ..default()
            });
        }
    }
    // unlit sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(
            Mesh::try_from(shape::Icosphere {
                radius: 0.45,
                subdivisions: 32,
            })
            .unwrap(),
        ),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("#ffd891").unwrap(),
            // vary key PBR parameters on a grid of spheres to show the effect
            unlit: true,
            ..default()
        }),
        transform: Transform::from_xyz(-5.0, -2.5, 0.0),
        ..default()
    });

    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(50.0, 50.0, 50.0),
        point_light: PointLight {
            intensity: 600000.,
            range: 100.,
            ..default()
        },
        ..default()
    });

    // labels
    commands.spawn(
        TextBundle::from_section(
            "Perceptual Roughness",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 36.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(20.0),
            left: Val::Px(100.0),
            ..default()
        }),
    );

    commands.spawn(TextBundle {
        text: Text::from_section(
            "Metallic",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 36.0,
                color: Color::WHITE,
            },
        ),
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(130.0),
            right: Val::Px(0.0),
            ..default()
        },
        transform: Transform {
            rotation: Quat::from_rotation_z(std::f32::consts::PI / 2.0),
            ..default()
        },
        ..default()
    });

    commands.spawn((
        TextBundle::from_section(
            "Loading Environment Map...",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 36.0,
                color: Color::RED,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(20.0),
            right: Val::Px(20.0),
            ..default()
        }),
        EnvironmentMapLabel,
    ));

    // camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::default(), Vec3::Y),
            projection: OrthographicProjection {
                scale: 0.01,
                ..default()
            }
            .into(),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        },
    ));
}

fn environment_map_load_finish(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    environment_maps: Query<&EnvironmentMapLight>,
    label_query: Query<Entity, With<EnvironmentMapLabel>>,
) {
    if let Ok(environment_map) = environment_maps.get_single() {
        if asset_server.get_load_state(&environment_map.diffuse_map) == LoadState::Loaded
            && asset_server.get_load_state(&environment_map.specular_map) == LoadState::Loaded
        {
            if let Ok(label_entity) = label_query.get_single() {
                commands.entity(label_entity).despawn();
            }
        }
    }
}

#[derive(Component)]
struct EnvironmentMapLabel;
