//! Demonstrates realtime dynamic raytraced lighting using Bevy Solari.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use argh::FromArgs;
use bevy::{
    prelude::*,
    render::{camera::CameraMainTextureUsages, mesh::Indices, render_resource::TextureUsages},
    scene::SceneInstanceReady,
    solari::{
        pathtracer::{Pathtracer, PathtracingPlugin},
        prelude::{RaytracingMesh3d, SolariLighting, SolariPlugins},
    },
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

/// `bevy_solari` demo.
#[derive(FromArgs, Resource, Clone, Copy)]
struct Args {
    /// use the reference pathtracer instead of the realtime lighting system.
    #[argh(switch)]
    pathtracer: Option<bool>,
}

fn main() {
    let args: Args = argh::from_env();

    let mut app = App::new();
    app.add_plugins((DefaultPlugins, SolariPlugins, CameraControllerPlugin))
        .insert_resource(args)
        .add_systems(Startup, setup);

    if args.pathtracer == Some(true) {
        app.add_plugins(PathtracingPlugin);
    }

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, args: Res<Args>) {
    commands
        .spawn(SceneRoot(asset_server.load(
            GltfAssetLabel::Scene(0).from_asset("models/CornellBox/CornellBox.glb"),
        )))
        .observe(add_raytracing_meshes_on_scene_load);

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: false, // Solari replaces shadow mapping
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI * -0.43, PI * -0.08, 0.0)),
    ));

    let mut camera = commands.spawn((
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
        Transform::from_xyz(-278.0, 273.0, 800.0),
        // Msaa::Off and CameraMainTextureUsages with STORAGE_BINDING are required for Solari
        CameraMainTextureUsages::default().with(TextureUsages::STORAGE_BINDING),
        Msaa::Off,
    ));
    if args.pathtracer == Some(true) {
        camera.insert(Pathtracer::default());
    } else {
        camera.insert(SolariLighting::default());
    }
}

fn add_raytracing_meshes_on_scene_load(
    trigger: On<SceneInstanceReady>,
    children: Query<&Children>,
    mesh: Query<&Mesh3d>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    args: Res<Args>,
) {
    // Ensure meshes are bevy_solari compatible
    for (_, mesh) in meshes.iter_mut() {
        mesh.remove_attribute(Mesh::ATTRIBUTE_UV_1.id);
        mesh.remove_attribute(Mesh::ATTRIBUTE_COLOR.id);
        mesh.generate_tangents().unwrap();

        if let Some(indices) = mesh.indices_mut() {
            if let Indices::U16(u16_indices) = indices {
                *indices = Indices::U32(u16_indices.iter().map(|i| *i as u32).collect());
            }
        }
    }

    // Add raytracing mesh handles
    for descendant in children.iter_descendants(trigger.target()) {
        if let Ok(mesh) = mesh.get(descendant) {
            commands
                .entity(descendant)
                .insert(RaytracingMesh3d(mesh.0.clone()));

            if args.pathtracer == Some(true) {
                commands.entity(descendant).remove::<Mesh3d>();
            }
        }
    }

    // Increase material emissive intensity to make it prettier for the example
    for (_, material) in materials.iter_mut() {
        material.emissive *= 200.0;
    }
}
