//! A minimal example showing the steps needed to get started with the plugin.
//!
//! Controls:
//! - Pan: Left mouse button + drag
//! - Orbit: Right mouse button + drag
//! - Zoom: Mouse wheel
//!
//! This controller relies on picking to determine what point to orbit around and zoom towards:
//! your motion will be relative to the point under the mouse cursor.

mod pan_orbit_camera_custom_input_plugin;

use bevy::camera_controller::pan_orbit_camera::prelude::*;
use bevy::prelude::*;

use crate::pan_orbit_camera_custom_input_plugin::CustomInputPlugin;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshPickingPlugin, // Step 0: enable some picking backends for hit detection
            DefaultPanOrbitCameraPlugins, // Step 1: Add camera controller plugin
            CustomInputPlugin, // Step 1.5: Connect the camera controller to your inputs.
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        PanOrbitCamera::default(), // Step 2: add camera controller component to any cameras
        EnvironmentMapLight {
            // EnvironmentMapLight is optional and can be replaced with usual light.
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2500.0,
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 1.0),
    ));
    commands.spawn(WorldAssetRoot(
        asset_server.load("https://raw.githubusercontent.com/bevyengine/bevy_asset_files/refs/heads/main/PlaneEngine/scene.gltf#Scene0"),
    ));
}
