//! Demonstrates how to rotate the skybox and the environment map simultaneously.

use std::f32::consts::PI;

use bevy::{
    color::palettes::css::{GOLD, WHITE},
    core_pipeline::{tonemapping::Tonemapping::AcesFitted, Skybox},
    pbr::{CascadeShadowConfig, Cascades, CascadesVisibleEntities},
    prelude::*,
    render::{primitives::CascadesFrusta, texture::ImageLoaderSettings},
};

/// Entry point.
pub fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, rotate_skybox_and_environment_map)
        .run();
}

/// Initializes the scene.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    let sphere_mesh = create_sphere_mesh(&mut meshes);
    spawn_sphere(&mut commands, &mut materials, &asset_server, &sphere_mesh);
    spawn_light(&mut commands);
    spawn_camera(&mut commands, &asset_server);
}

/// Rotate the skybox and the environment map per frame.
fn rotate_skybox_and_environment_map(
    mut environments: Query<(&mut Skybox, &mut EnvironmentMapLight)>,
    time: Res<Time>,
) {
    let now = time.elapsed_seconds();
    let rotation = Quat::from_rotation_y(0.2 * now);
    for (mut skybox, mut environment_map) in environments.iter_mut() {
        skybox.rotation = rotation;
        environment_map.rotation = rotation;
    }
}

/// Generates a sphere.
fn create_sphere_mesh(meshes: &mut Assets<Mesh>) -> Handle<Mesh> {
    // We're going to use normal maps, so make sure we've generated tangents, or
    // else the normal maps won't show up.

    let mut sphere_mesh = Sphere::new(1.0).mesh().build();
    sphere_mesh
        .generate_tangents()
        .expect("Failed to generate tangents");
    meshes.add(sphere_mesh)
}

/// Spawn a regular object with a clearcoat layer. This looks like car paint.
fn spawn_sphere(
    commands: &mut Commands,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    sphere_mesh: &Handle<Mesh>,
) {
    commands.spawn(PbrBundle {
        mesh: sphere_mesh.clone(),
        material: materials.add(StandardMaterial {
            clearcoat: 1.0,
            clearcoat_perceptual_roughness: 0.3,
            clearcoat_normal_texture: Some(asset_server.load_with_settings(
                "textures/ScratchedGold-Normal.png",
                |settings: &mut ImageLoaderSettings| settings.is_srgb = false,
            )),
            metallic: 0.9,
            perceptual_roughness: 0.1,
            base_color: GOLD.into(),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.0, 0.0).with_scale(Vec3::splat(1.25)),
        ..default()
    });
}

/// Spawns a light.
fn spawn_light(commands: &mut Commands) {
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            color: WHITE.into(),
            intensity: 100000.0,
            ..default()
        },
        ..default()
    });
}

/// Spawns a camera with associated skybox and environment map.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer) {
    commands
        .spawn(Camera3dBundle {
            camera: Camera {
                hdr: true,
                ..default()
            },
            projection: Projection::Perspective(PerspectiveProjection {
                fov: 27.0 / 180.0 * PI,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.0, 10.0),
            tonemapping: AcesFitted,
            ..default()
        })
        .insert(Skybox {
            brightness: 5000.0,
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            ..default()
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        });
}
