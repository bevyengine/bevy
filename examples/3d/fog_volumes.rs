//! Demonstrates fog volumes with voxel density textures.
//!
//! We render the Stanford bunny as a fog volume. Parts of the bunny become
//! lighter and darker as the camera rotates. This is physically-accurate
//! behavior that results from the scattering and absorption of the directional
//! light.

use bevy::{
    math::vec3,
    pbr::{FogVolume, VolumetricFogSettings, VolumetricLight},
    prelude::*,
};

/// Entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Fog Volumes Example".into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(AmbientLight::NONE)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_camera)
        .run();
}

/// Spawns all the objects in the scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn a fog volume with a voxelized version of the Stanford bunny.
    commands
        .spawn(SpatialBundle {
            visibility: Visibility::Visible,
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        })
        .insert(FogVolume {
            density_texture: Some(asset_server.load("volumes/bunny.ktx2")),
            density_factor: 1.0,
            // Scatter as much of the light as possible, to brighten the bunny
            // up.
            scattering: 1.0,
            ..default()
        });

    // Spawn a bright directional light that illuminates the fog well.
    commands
        .spawn(DirectionalLightBundle {
            transform: Transform::from_xyz(1.0, 1.0, -0.3).looking_at(vec3(0.0, 0.5, 0.0), Vec3::Y),
            directional_light: DirectionalLight {
                shadows_enabled: true,
                illuminance: 32000.0,
                ..default()
            },
            ..default()
        })
        // Make sure to add this for the light to interact with the fog.
        .insert(VolumetricLight);

    // Spawn a camera.
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(-0.75, 1.0, 2.0)
                .looking_at(vec3(0.0, 0.0, 0.0), Vec3::Y),
            camera: Camera {
                hdr: true,
                ..default()
            },
            ..default()
        })
        .insert(VolumetricFogSettings {
            // Make this relatively high in order to increase the fog quality.
            step_count: 64,
            // Disable ambient light.
            ambient_intensity: 0.0,
            ..default()
        });
}

/// Rotates the camera a bit every frame.
fn rotate_camera(mut cameras: Query<&mut Transform, With<Camera3d>>) {
    for mut camera_transform in cameras.iter_mut() {
        *camera_transform =
            Transform::from_translation(Quat::from_rotation_y(0.01) * camera_transform.translation)
                .looking_at(vec3(0.0, 0.5, 0.0), Vec3::Y);
    }
}
