//! This example demonstrates how to load FBX files using the `bevy_fbx` crate.
//!
//! The example loads a simple cube model from an FBX file and displays it
//! with proper lighting and shadows. The cube should rotate in the scene.

use bevy::{
    fbx::FbxAssetLabel,
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    prelude::*,
};
use std::f32::consts::*;

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, animate_light_direction)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        Camera3d::default(),
        // Transform::from_xyz(0.7, 2.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 550.0,
            ..default()
        },
    ));

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        // This is a relatively small scene, so use tighter shadow
        // cascade bounds than the default for better quality.
        // We also adjusted the shadow map to be larger since we're
        // only using a single cascade.
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 1.6,
            ..default()
        }
        .build(),
    ));

    // Load the FBX file and spawn its first scene
    commands.spawn(SceneRoot(
        asset_server.load(FbxAssetLabel::Scene(0).from_asset("models/cube/cube.fbx")),
    ));
    // commands.spawn(SceneRoot(
    //     asset_server.load(FbxAssetLabel::Scene(0).from_asset("models/nurbs_saddle.fbx")),
    // ));
    // commands.spawn(SceneRoot(
    //     asset_server.load(FbxAssetLabel::Scene(0).from_asset("models/cube_anim.fbx")),
    // ));
}

fn animate_light_direction(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<DirectionalLight>>,
) {
    for mut transform in &mut query {
        transform.rotation = Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            time.elapsed_secs() * PI / 5.0,
            -FRAC_PI_4,
        );
    }
}
