//! This example showcases the default freecam camera controller.
//! 
//! a

use std::f32::consts::{FRAC_PI_4, PI};

use bevy::{
    camera_controller::free_cam::{FreeCam, FreeCamPlugin},
    color::palettes::tailwind,
    prelude::*
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Plugin that enables freecam functionality
        .add_plugins(FreeCamPlugin)
        // Example code plugins
        .add_plugins((CameraPlugin, ScenePlugin))
        .run();
}

// Plugin that spawns the camera.
struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::X, Vec3::Y),
        // This component sotres all camera settings and state, which is used by the FreeCamPlugin to
        // control it. These properties can be chagned at runtime, but beware the controller system is
        // constantly using and modifying those values unless the enabled field is false.
        FreeCam {
            sensitivity: 0.1,
            walk_speed: 3.0,
            run_speed: 9.0,
            ..default()
        },
    ));
}

// Plugin that spawns the scene and lighting.
struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                Startup,
                (spawn_lights, spawn_world));
    }
}

fn spawn_lights(mut commands: Commands) {
    commands.spawn((
        PointLight {
            color: Color::from(tailwind::ORANGE_100),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));

    commands.spawn((
        PointLight {
            color: Color::from(tailwind::RED_800),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, -0.5, 0.0),
    ));
}

fn spawn_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)));
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));

    let floor_material = materials.add(Color::WHITE);

    // Top side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(floor_material.clone()),
    ));
    // Under side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(floor_material.clone()),
        Transform::default().with_rotation(Quat::from_rotation_x(PI))
    ));
    // Hidden cube under floor
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(floor_material.clone()),
        Transform {
            translation: Vec3::new(0.0, -2.0, 0.0),
            rotation: Quat::from_euler(EulerRot::YXZEx, FRAC_PI_4, FRAC_PI_4, 0.0),
            ..default()
        }
    ));
}
