//! Meshlet rendering for dense high-poly scenes (experimental).

// Note: This example showcases the meshlet API, but is not the type of scene that would benefit from using meshlets.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    pbr::{
        experimental::meshlet::{MaterialMeshletMeshBundle, MeshletPlugin},
        CascadeShadowConfigBuilder, DirectionalLightShadowMap,
    },
    prelude::*,
    render::render_resource::AsBindGroup,
    window::PresentMode,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::{f32::consts::PI, path::Path, process::ExitCode};

const ASSET_URL: &str =
    "https://raw.githubusercontent.com/JMS55/bevy_meshlet_asset/b6c712cfc87c65de419f856845401aba336a7bcd/bunny.meshlet_mesh";

fn main() -> ExitCode {
    if !Path::new("./assets/models/bunny.meshlet_mesh").exists() {
        eprintln!("ERROR: Asset at path <bevy>/assets/models/bunny.meshlet_mesh is missing. Please download it from {ASSET_URL}");
        return ExitCode::FAILURE;
    }

    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            MeshletPlugin {
                cluster_buffer_slots: 700_000,
            },
            MaterialPlugin::<MeshletDebugMaterial>::default(),
            CameraControllerPlugin,
        ))
        .add_systems(Startup, setup)
        .run();

    ExitCode::SUCCESS
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut standard_materials: ResMut<Assets<StandardMaterial>>,
    mut debug_materials: ResMut<Assets<MeshletDebugMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(9.662523, 2.1472275, -0.5368076))
                .with_rotation(Quat::from_xyzw(
                    -0.07487535,
                    0.7221289,
                    0.07915055,
                    0.6831241,
                )),
            msaa: Msaa::Off,
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 150.0,
            ..default()
        },
        CameraController::default(),
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: light_consts::lux::FULL_DAYLIGHT,
            shadows_enabled: false,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            num_cascades: 1,
            maximum_distance: 15.0,
            ..default()
        }
        .build(),
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            PI * -0.15,
            PI * -0.15,
        )),
        ..default()
    });

    // A custom file format storing a [`bevy_render::mesh::Mesh`]
    // that has been converted to a [`bevy_pbr::meshlet::MeshletMesh`]
    // using [`bevy_pbr::meshlet::MeshletMesh::from_mesh`], which is
    // a function only available when the `meshlet_processor` cargo feature is enabled.
    let meshlet_mesh_handle = asset_server.load("models/bunny.meshlet_mesh");
    let debug_material = debug_materials.add(MeshletDebugMaterial::default());

    for x in -10..=10 {
        for z in -10..=10 {
            for y in -3..=3 {
                commands.spawn(MaterialMeshletMeshBundle {
                    meshlet_mesh: meshlet_mesh_handle.clone(),
                    material: debug_material.clone(),
                    transform: Transform::default()
                        .with_scale(Vec3::splat(0.2))
                        .with_rotation(Quat::from_rotation_z(PI))
                        .with_rotation(Quat::from_rotation_y(PI))
                        .with_translation(Vec3::new(
                            x as f32 / 2.0,
                            y as f32 / 2.0,
                            z as f32 / 2.0,
                        )),
                    ..default()
                });
            }
        }
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
struct MeshletDebugMaterial {
    _dummy: (),
}
impl Material for MeshletDebugMaterial {}
