//! Light probe blending example.

use bevy::{camera::Hdr, color::palettes::css::WHITE, prelude::*};

#[derive(Clone, Copy, Component, Debug)]
struct ReflectiveSphere;

const SPHERE_MOVEMENT_SPEED: f32 = 0.3;
const CAMERA_OFFSET: Vec3 = vec3(2.0, 2.0, 2.0);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_sphere, make_camera_follow_sphere).chain())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        Hdr,
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 5000.0,
            ..default()
        },
    ));

    commands.spawn((
        DirectionalLight {
            color: WHITE.into(),
            illuminance: 10000.0,
            ..default()
        },
        Transform::from_translation(CAMERA_OFFSET).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/two_rooms.glb")),
    ));

    let sphere = meshes.add(Sphere::default().mesh().uv(32, 18));
    let material = materials.add(StandardMaterial {
        base_color: WHITE.into(),
        metallic: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    });

    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        ReflectiveSphere,
    ));

    commands.spawn((
        LightProbe {
            falloff: Vec3::splat(0.5),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("textures/light_probe_blending_example/diffuse_room1.ktx2"),
            specular_map: asset_server
                .load("textures/light_probe_blending_example/specular_room1.ktx2"),
            intensity: 5000.0,
            ..default()
        },
        Transform::from_scale(Vec3::splat(15.0)),
    ));

    commands.spawn((
        LightProbe {
            falloff: Vec3::splat(0.5),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("textures/light_probe_blending_example/diffuse_room2.ktx2"),
            specular_map: asset_server
                .load("textures/light_probe_blending_example/specular_room2.ktx2"),
            intensity: 5000.0,
            ..default()
        },
        Transform::from_scale(Vec3::splat(15.0)).with_translation(vec3(0.0, 0.0, -11.0)),
    ));
}

fn move_sphere(mut spheres: Query<&mut Transform, With<ReflectiveSphere>>, time: Res<Time>) {
    let Some(t) = SmoothStepCurve
        .ping_pong()
        .unwrap()
        .forever()
        .unwrap()
        .sample(time.elapsed_secs() * SPHERE_MOVEMENT_SPEED)
    else {
        return;
    };
    for mut sphere_transform in &mut spheres {
        sphere_transform.translation.z = -11.0 * t;
    }
}

fn make_camera_follow_sphere(
    mut cameras: Query<&mut Transform, (With<Camera>, Without<ReflectiveSphere>)>,
    spheres: Query<&Transform, With<ReflectiveSphere>>,
) {
    let Some(sphere_transform) = spheres.iter().next() else {
        return;
    };
    for mut camera_transform in &mut cameras {
        camera_transform.translation = sphere_transform.translation + CAMERA_OFFSET;
    }
}
