//! Demonstrates realtime dynamic global illumination rendering using Bevy Solari.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    prelude::*,
    render::mesh::Indices,
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
        .spawn(SceneRoot(
            asset_server.load("models/CornellBox/box_modified.glb#Scene0"),
        ))
        .observe(add_raytracing_meshes_on_scene_load);

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI * -0.43, PI * -0.08, 0.0)),
    ));

    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        CameraController::default(),
        Pathtracer,
        Transform::from_matrix(Mat4 {
            x_axis: Vec4::new(0.99480534, 0.0, -0.10179563, 0.0),
            y_axis: Vec4::new(-0.019938117, 0.98063105, -0.19484669, 0.0),
            z_axis: Vec4::new(0.09982395, 0.19586414, 0.975537, 0.0),
            w_axis: Vec4::new(0.68394995, 2.2785425, 6.68395, 1.0),
        }),
    ));
}

fn add_raytracing_meshes_on_scene_load(
    trigger: Trigger<SceneInstanceReady>,
    children: Query<&Children>,
    mesh: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    for (_, mesh) in meshes.iter_mut() {
        if let Some(indices) = mesh.indices_mut() {
            if let Indices::U16(u16_indices) = indices {
                *indices = Indices::U32(u16_indices.iter().map(|i| *i as u32).collect());
            }
        }
    }

    for descendant in children.iter_descendants(trigger.target()) {
        if let Ok(mesh) = mesh.get(descendant) {
            commands
                .entity(descendant)
                .insert(RaytracingMesh3d(mesh.0.clone()));
        }
    }
}
