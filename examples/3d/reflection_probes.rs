//! This example shows how to place reflection probes in the scene.

use bevy::math::Vec3A;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Create a sphere mesh.
    let sphere_mesh = meshes.add(
        Mesh::try_from(shape::Icosphere {
            radius: 0.45,
            ..default()
        })
        .unwrap(),
    );

    // Create the left sphere.
    let left_sphere = commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("#ffd891").unwrap(),
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..StandardMaterial::default()
        }),
        transform: Transform::from_xyz(-2.5, 0.0, 0.0),
        ..PbrBundle::default()
    });

    // Create the right sphere.
    let right_sphere = commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: materials.add(StandardMaterial {
            base_color: Color::hex("#ffd891").unwrap(),
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..StandardMaterial::default()
        }),
        transform: Transform::from_xyz(2.5, 0.0, 0.0),
        ..PbrBundle::default()
    });

    // Create the light.
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(50.0, 50.0, 50.0),
        point_light: PointLight {
            intensity: 600000.,
            range: 100.,
            ..default()
        },
        ..default()
    });

    // Create the left reflection probe.
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(-2.5, 0.0, 0.0),
            ..SpatialBundle::default()
        },
        LightProbe {
            half_extents: Vec3A::splat(2.5),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        },
    ));

    // Create the right reflection probe.
    commands.spawn((
        SpatialBundle {
            transform: Transform::from_xyz(2.5, 0.0, 0.0),
            ..SpatialBundle::default()
        },
        LightProbe {
            half_extents: Vec3A::splat(2.5),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/footprint_court_diffuse.ktx2"),
            specular_map: asset_server.load("environment_maps/footprint_court_specular.ktx2"),
        },
    ));

    // Create the camera.
    commands.spawn((Camera3dBundle {
        transform: Transform::from_xyz(0.0, 0.0, 8.0).looking_at(Vec3::default(), Vec3::Y),
        projection: OrthographicProjection {
            scale: 0.01,
            ..default()
        }
        .into(),
        ..default()
    },));
}
