//! Decal rendering.
//! Note: On Wasm, this example only runs on WebGPU

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    core_pipeline::prepass::DepthPrepass,
    pbr::decal::{ForwardDecal, ForwardDecalMaterial, ForwardDecalMaterialExt},
    prelude::*,
};
use chacha20::ChaCha8Rng;
use rand::{RngExt, SeedableRng};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut asset_commands: AssetCommands,
    asset_server: Res<AssetServer>,
) {
    // Spawn the forward decal
    commands.spawn((
        Name::new("Decal"),
        ForwardDecal,
        MeshMaterial3d(asset_commands.spawn_asset(ForwardDecalMaterial {
            base: StandardMaterial {
                base_color_texture: Some(asset_server.load("textures/uv_checker_bw.png")),
                ..default()
            },
            extension: ForwardDecalMaterialExt {
                depth_fade_factor: 1.0,
            },
        })),
        Transform::from_scale(Vec3::splat(4.0)),
    ));

    commands.spawn((
        Name::new("Camera"),
        Camera3d::default(),
        FreeCamera::default(),
        // Must enable the depth prepass to render forward decals
        DepthPrepass,
        Transform::from_xyz(2.0, 9.5, 2.5).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let white_material = asset_commands.spawn_asset(StandardMaterial::from(Color::WHITE));

    commands.spawn((
        Name::new("Floor"),
        Mesh3d(asset_commands.spawn_asset(Mesh::from(Rectangle::from_length(10.0)))),
        MeshMaterial3d(white_material.clone()),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // Spawn a few cube with random rotations to showcase how the decals behave with non-flat geometry
    let num_obs = 10;
    let mut rng = ChaCha8Rng::seed_from_u64(19878367467713);
    for i in 0..num_obs {
        for j in 0..num_obs {
            let rotation_axis: [f32; 3] = rng.random();
            let rotation_vec: Vec3 = rotation_axis.into();
            let rotation: u32 = rng.random_range(0..360);
            let transform = Transform::from_xyz(
                (-num_obs + 1) as f32 / 2.0 + i as f32,
                -0.2,
                (-num_obs + 1) as f32 / 2.0 + j as f32,
            )
            .with_rotation(Quat::from_axis_angle(
                rotation_vec.normalize_or_zero(),
                (rotation as f32).to_radians(),
            ));

            commands.spawn((
                Mesh3d(asset_commands.spawn_asset(Mesh::from(Cuboid::from_length(0.6)))),
                MeshMaterial3d(white_material.clone()),
                transform,
            ));
        }
    }

    commands.spawn((
        Name::new("Light"),
        PointLight {
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
}
