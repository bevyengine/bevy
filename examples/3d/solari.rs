//! Demonstrates realtime dynamic global illumination rendering using Bevy Solari.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    prelude::*,
    render::{camera::CameraMainTextureUsages, mesh::Indices, render_resource::TextureUsages},
    scene::SceneInstanceReady,
    solari::{
        pathtracer::Pathtracer,
        prelude::{RaytracingMesh3d, SolariPlugin},
    },
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SolariPlugin, CameraControllerPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands
        .spawn(SceneRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("models/CornellBox/CornellBox.glb"),
        )))
        .observe(add_raytracing_meshes_on_scene_load);

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI * -0.43, PI * -0.08, 0.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        CameraController {
            walk_speed: 500.0,
            run_speed: 1500.0,
            ..Default::default()
        },
        Pathtracer::default(),
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
        Transform::from_xyz(-278.0, 273.0, 800.0),
    ));
}

fn add_raytracing_meshes_on_scene_load(
    trigger: On<SceneInstanceReady>,
    children: Query<&Children>,
    mesh: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    // Ensure meshes are bery_solari compatible
    for (_, mesh) in meshes.iter_mut() {
        mesh.remove_attribute(Mesh::ATTRIBUTE_UV_1.id);
        mesh.generate_tangents().unwrap();

        if let Some(indices) = mesh.indices_mut() {
            if let Indices::U16(u16_indices) = indices {
                *indices = Indices::U32(u16_indices.iter().map(|i| *i as u32).collect());
            }
        }
    }

    for descendant in children.iter_descendants(trigger.target().unwrap()) {
        if let Ok(mesh) = mesh.get(descendant) {
            commands
                .entity(descendant)
                .insert(RaytracingMesh3d(mesh.0.clone()))
                .remove::<Mesh3d>();
        }
    }
}
