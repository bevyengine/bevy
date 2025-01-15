//! This example showcases pbr atmospheric scattering

use std::f32::consts::PI;

use bevy::{
    core_pipeline::tonemapping::Tonemapping,
    pbr::{Atmosphere, AtmosphereSettings, CascadeShadowConfigBuilder},
    prelude::*,
};
use light_consts::lux;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup_camera_fog, setup_terrain_scene))
        .add_systems(Update, dynamic_scene)
        .run();
}

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Camera {
            hdr: true,
            ..default()
        },
        Tonemapping::AcesFitted,
        Transform::from_xyz(-1.2, 0.15, 0.0).looking_at(Vec3::Y * 0.1, Vec3::Y),
        Atmosphere::EARTH,
        AtmosphereSettings {
            scene_units_to_km: 1.0,
            ..Default::default()
        },
    ));
}

#[derive(Component)]
struct Terrain;

fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 3.0,
        ..default()
    }
    .build();

    // Sun
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.98, 0.95, 0.82),
            shadows_enabled: true,
            illuminance: lux::AMBIENT_DAYLIGHT,
            ..default()
        },
        Transform::from_xyz(1.0, -1.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        cascade_shadow_config,
    ));

    // light probe spheres
    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Sphere {
            radius: 1.0,
            ..default()
        }))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, -0.1).with_scale(Vec3::splat(0.05)),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Mesh::from(Sphere {
            radius: 1.0,
            ..default()
        }))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 0.0,
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, 0.1).with_scale(Vec3::splat(0.05)),
    ));

    // Terrain
    commands.spawn((
        Terrain,
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/terrain/terrain.glb")),
        ),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
    ));
}

fn dynamic_scene(
    mut sun: Single<&mut Transform, With<DirectionalLight>>,
    mut terrain: Single<&mut Transform, (With<Terrain>, Without<DirectionalLight>)>,
    time: Res<Time>,
) {
    let t = time.elapsed_secs() * 0.5;
    let radius = 0.3;
    let x = radius * t.cos();
    let y = radius * t.sin();
    sun.translation = Vec3::new(1.0, y + 0.15, x);
    sun.look_at(Vec3::ZERO, Vec3::Y);
}
