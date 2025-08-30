//! Meshlet rendering for dense high-poly scenes (experimental).

// Note: This example showcases the meshlet API, but is not the type of scene that would benefit from using meshlets.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap},
    pbr::experimental::meshlet::{MeshletMesh3d, MeshletPlugin},
    prelude::*,
    render::render_resource::AsBindGroup,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

const ASSET_URL: &str =
    "https://github.com/bevyengine/bevy_asset_files/raw/9bf88c42b9d06a3634eed633d90ce5fab02c31da/meshlet/bunny.meshlet_mesh";

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins((
            DefaultPlugins,
            MeshletPlugin {
                cluster_buffer_slots: 1 << 14,
            },
            MaterialPlugin::<MeshletDebugMaterial>::default(),
            CameraControllerPlugin,
        ))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut debug_materials: ResMut<Assets<MeshletDebugMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_translation(Vec3::new(1.8, 0.4, -0.1)).looking_at(Vec3::ZERO, Vec3::Y),
        Msaa::Off,
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 150.0,
            ..default()
        },
        CameraController::default(),
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 15.0,
            ..default()
        }
        .build(),
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI * -0.15, PI * -0.15)),
    ));

    // A custom file format storing a [`bevy_mesh::Mesh`]
    // that has been converted to a [`bevy_pbr::meshlet::MeshletMesh`]
    // using [`bevy_pbr::meshlet::MeshletMesh::from_mesh`], which is
    // a function only available when the `meshlet_processor` cargo feature is enabled.
    let meshlet_mesh_handle = asset_server.load(ASSET_URL);
    let debug_material = debug_materials.add(MeshletDebugMaterial::default());

    for x in -2..=2 {
        commands.spawn((
            MeshletMesh3d(meshlet_mesh_handle.clone()),
            MeshMaterial3d(standard_materials.add(StandardMaterial {
                base_color: match x {
                    -2 => Srgba::hex("#dc2626").unwrap().into(),
                    -1 => Srgba::hex("#ea580c").unwrap().into(),
                    0 => Srgba::hex("#facc15").unwrap().into(),
                    1 => Srgba::hex("#16a34a").unwrap().into(),
                    2 => Srgba::hex("#0284c7").unwrap().into(),
                    _ => unreachable!(),
                },
                perceptual_roughness: (x + 2) as f32 / 4.0,
                ..default()
            })),
            Transform::default()
                .with_scale(Vec3::splat(0.2))
                .with_translation(Vec3::new(x as f32 / 2.0, 0.0, -0.3)),
        ));
    }
    for x in -2..=2 {
        commands.spawn((
            MeshletMesh3d(meshlet_mesh_handle.clone()),
            MeshMaterial3d(debug_material.clone()),
            Transform::default()
                .with_scale(Vec3::splat(0.2))
                .with_rotation(Quat::from_rotation_y(PI))
                .with_translation(Vec3::new(x as f32 / 2.0, 0.0, 0.3)),
        ));
    }

    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(standard_materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 1.0,
            ..default()
        })),
    ));
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
struct MeshletDebugMaterial {
    _dummy: (),
}

impl Material for MeshletDebugMaterial {}
