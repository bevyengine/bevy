//! GPU-driven meshlet-based rendering.

#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use bevy::{
    pbr::{
        meshlet::{MaterialMeshletMeshBundle, MeshletPlugin},
        CascadeShadowConfigBuilder,
    },
    prelude::*,
    render::render_resource::AsBindGroup,
};
use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            MeshletPlugin,
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
) {
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_translation(Vec3::new(1.8, 0.4, -0.1))
                .looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
        },
        CameraController::default(),
    ));

    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: true,
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

    let meshlet_mesh_handle = asset_server.load("models/bunny.meshlet_mesh");
    let debug_material = debug_materials.add(MeshletDebugMaterial::default());

    for x in -2..=2 {
        commands.spawn(MaterialMeshletMeshBundle {
            meshlet_mesh: meshlet_mesh_handle.clone(),
            material: standard_materials.add(StandardMaterial {
                base_color: match x {
                    -2 => Color::hex("#dc2626").unwrap(),
                    -1 => Color::hex("#ea580c").unwrap(),
                    0 => Color::hex("#facc15").unwrap(),
                    1 => Color::hex("#16a34a").unwrap(),
                    2 => Color::hex("#0284c7").unwrap(),
                    _ => unreachable!(),
                },
                perceptual_roughness: (x + 2) as f32 / 4.0,
                ..default()
            }),
            transform: Transform::default()
                .with_scale(Vec3::splat(0.2))
                .with_translation(Vec3::new(x as f32 / 2.0, 0.0, -0.3)),
            ..default()
        });
    }
    for x in -2..=2 {
        commands.spawn(MaterialMeshletMeshBundle {
            meshlet_mesh: meshlet_mesh_handle.clone(),
            material: debug_material.clone(),
            transform: Transform::default()
                .with_scale(Vec3::splat(0.2))
                .with_rotation(Quat::from_rotation_y(PI))
                .with_translation(Vec3::new(x as f32 / 2.0, 0.0, 0.3)),
            ..default()
        });
    }
}

#[derive(Asset, TypePath, AsBindGroup, Clone, Default)]
struct MeshletDebugMaterial {
    _dummy: (),
}
impl Material for MeshletDebugMaterial {}
