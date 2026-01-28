//! Demonstrates blending between multiple reflection probes.
//!
//! This example shows a reflective sphere that moves between two rooms, each of
//! which contains a reflection probe with a falloff range. Bevy performs a
//! blend between the two reflection probes as the sphere moves.

use std::f32::consts::{FRAC_PI_4, PI};

use bevy::{
    camera::Hdr,
    color::palettes::css::WHITE,
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    math::ops::{cos, sin},
    prelude::*,
};

/// A marker component for the reflective sphere.
#[derive(Clone, Copy, Component, Debug)]
struct ReflectiveSphere;

/// The speed at which the sphere moves, as a ratio of the total distance it
/// travels to seconds.
///
/// Specifically, the value of 0.3 means that it moves 3/10 of the way to the
/// other side per second.
const SPHERE_MOVEMENT_SPEED: f32 = 0.3;

/// The number of meters that separates the center of each room.
const ROOM_SEPARATION: f32 = 11.0;

/// The side length of the light probe cube, in meters.
const LIGHT_PROBE_SIDE_LENGTH: f32 = 15.0;

/// The distance over which the light probe fades out, expressed as a fraction
/// of the side length of the probe.
const LIGHT_PROBE_FALLOFF: f32 = 0.5;

/// The number of radians of inclination (pitch) that one pixel of mouse
/// movement corresponds to.
const CAMERA_ORBIT_SPEED_INCLINATION: f32 = 0.003;

/// The number of radians of azumith (yaw) that one pixel of mouse movement
/// corresponds to.
const CAMERA_ORBIT_SPEED_AZIMUTH: f32 = 0.004;

/// The number of meters that one line of mouse scroll corresponds to.
const CAMERA_ZOOM_SPEED: f32 = 0.15;

/// Information about the orbital pan/zoom camera.
///
/// These are in [spherical coordinates].
///
/// [spherical coordinates]: https://en.wikipedia.org/wiki/Spherical_coordinate_system
#[derive(Component)]
struct OrbitCamera {
    /// The distance between the camera and the sphere, in meters.
    radius: f32,
    /// The camera latitude in radians, relative to the sphere.
    inclination: f32,
    /// The camera longitude in radians, relative to the sphere.
    azimuth: f32,
}

/// The brightness of the light probe.
const LIGHT_PROBE_INTENSITY: f32 = 5000.0;

/// The entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Light Probe Blending Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_sphere, orbit_camera).chain())
        .run();
}

/// Performs initial setup of the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    spawn_camera(&mut commands);
    spawn_gltf_scene(&mut commands, &asset_server);
    spawn_reflective_sphere(&mut commands, &mut meshes, &mut materials);
    spawn_light_probes(&mut commands, &asset_server);
    spawn_help_text(&mut commands);
}

/// Spawns the orbital pan/zoom camera.
fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::IDENTITY,
        Hdr,
        OrbitCamera {
            radius: 3.0,
            inclination: 7.0 * FRAC_PI_4,
            azimuth: FRAC_PI_4,
        },
    ));
}

/// Spawns the glTF scene that contains the two rooms.
fn spawn_gltf_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/two_rooms.glb")),
    ));
}

/// Spawns the reflective sphere, creating its mesh and material in the process.
fn spawn_reflective_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    // Create a mesh.
    let sphere = meshes.add(Sphere::default().mesh().uv(32, 18));

    // Create a reflective material.
    let material = materials.add(StandardMaterial {
        base_color: WHITE.into(),
        metallic: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    });

    // Spawn the sphere.
    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        ReflectiveSphere,
    ));
}

/// Spawns the two light probes, one for each room.
fn spawn_light_probes(commands: &mut Commands, asset_server: &AssetServer) {
    // The cubemaps were baked with a different coordinate system than the
    // default Bevy one, so account for this.
    let light_probe_rotation = Quat::from_rotation_y(PI);

    // Spawn the first room's light probe.
    commands.spawn((
        LightProbe {
            falloff: Vec3::splat(LIGHT_PROBE_FALLOFF),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("textures/light_probe_blending_example/diffuse_room1.ktx2"),
            specular_map: asset_server
                .load("textures/light_probe_blending_example/specular_room1.ktx2"),
            intensity: LIGHT_PROBE_INTENSITY,
            rotation: light_probe_rotation,
            ..default()
        },
        Transform::from_scale(Vec3::splat(LIGHT_PROBE_SIDE_LENGTH)),
    ));

    // Spawn the second room's light probe.
    commands.spawn((
        LightProbe {
            falloff: Vec3::splat(LIGHT_PROBE_FALLOFF),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server
                .load("textures/light_probe_blending_example/diffuse_room2.ktx2"),
            specular_map: asset_server
                .load("textures/light_probe_blending_example/specular_room2.ktx2"),
            intensity: LIGHT_PROBE_INTENSITY,
            rotation: light_probe_rotation,
            ..default()
        },
        Transform::from_scale(Vec3::splat(LIGHT_PROBE_SIDE_LENGTH)).with_translation(vec3(
            0.0,
            0.0,
            -ROOM_SEPARATION,
        )),
    ));
}

/// Spawns the help text at the top of the screen.
fn spawn_help_text(commands: &mut Commands) {
    commands.spawn((
        Text::new("Click and drag to orbit the camera\nUse the mouse wheel to zoom"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// Moves the sphere a bit every frame.
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
        sphere_transform.translation.z = -ROOM_SEPARATION * t;
    }
}

/// Processes requests from the user to move the camera.
fn orbit_camera(
    mut cameras: Query<(&mut Transform, &mut OrbitCamera)>,
    spheres: Query<&Transform, (With<ReflectiveSphere>, Without<OrbitCamera>)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    // Grab the sphere transform.
    let Some(sphere_transform) = spheres.iter().next() else {
        return;
    };

    for (mut camera_transform, mut orbit_camera) in &mut cameras {
        // Only pan if the left mouse button is pressed.
        if mouse_buttons.pressed(MouseButton::Left) {
            let delta = mouse_motion.delta;
            orbit_camera.azimuth -= delta.x * CAMERA_ORBIT_SPEED_AZIMUTH;
            orbit_camera.inclination += delta.y * CAMERA_ORBIT_SPEED_INCLINATION;
        }

        // Zooming doesn't require a mouse button press, as it uses the mouse
        // wheel.
        orbit_camera.radius =
            (orbit_camera.radius - CAMERA_ZOOM_SPEED * mouse_scroll.delta.y).max(0.01);

        // Calculate the new translation using the [spherical coordinates
        // formula].
        //
        // [spherical coordinates formula]:
        // https://en.wikipedia.org/wiki/Spherical_coordinate_system#Cartesian_coordinates
        let new_translation = orbit_camera.radius
            * vec3(
                sin(orbit_camera.inclination) * cos(orbit_camera.azimuth),
                cos(orbit_camera.inclination),
                sin(orbit_camera.inclination) * sin(orbit_camera.azimuth),
            );

        // Write in the new transform.
        *camera_transform =
            Transform::from_translation(new_translation + sphere_transform.translation)
                .looking_at(sphere_transform.translation, Vec3::Y);
    }
}
