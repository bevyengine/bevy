//! This example shows how to configure Physically Based Rendering (PBR) parameters.

use bevy::{asset::LoadState, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(environment_map_load_finish)
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
                    base_color: Color::hex("ffd891").unwrap(),
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
            base_color: Color::hex("ffd891").unwrap(),
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
            position: UiRect {
                top: Val::Px(20.0),
                left: Val::Px(100.0),
                ..default()
            },
            ..default()
        }),
    );

    commands.spawn(
        TextBundle::from_section(
            "M\ne\nt\na\nl\nl\ni\nc",
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 36.0,
                color: Color::WHITE,
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: Val::Px(80.0),
                right: Val::Px(50.0),
                ..default()
            },
            ..default()
        }),
    );

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
            position: UiRect {
                bottom: Val::Px(20.0),
                right: Val::Px(20.0),
                ..default()
            },
            ..default()
        }),
        EnvironmentMapLabel,
    ));

    // camera
    let environment_map = EnvironmentMap {
        diffuse_map: asset_server.load("environment_maps/pisa_diffuse.ktx2"),
        specular_map: asset_server.load("environment_maps/pisa_specular.ktx2"),
    };
    commands.insert_resource(EnvironmentMapHandles {
        handles: environment_map.clone(),
    });
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
        environment_map,
    ));
}

fn environment_map_load_finish(
    mut commands: Commands,
    handles: Res<EnvironmentMapHandles>,
    asset_server: Res<AssetServer>,
    label_query: Query<Entity, With<EnvironmentMapLabel>>,
) {
    if asset_server.get_load_state(&handles.handles.diffuse_map) == LoadState::Loaded
        && asset_server.get_load_state(&handles.handles.specular_map) == LoadState::Loaded
    {
        if let Ok(label_entity) = label_query.get_single() {
            commands.entity(label_entity).despawn();
        }
    }
}

#[derive(Resource)]
struct EnvironmentMapHandles {
    handles: EnvironmentMap,
}

#[derive(Component)]
struct EnvironmentMapLabel;
