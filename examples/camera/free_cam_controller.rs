//! This example showcases the default freecam camera controller.
//!
//! The default freecam controller is useful for exploring large scenes, debugging and editing purposes. To use it,
//! simply add the [`FreeCamPlugin`] to your [`App`] and attach the [`FreeCam`] component to the camera entity you
//! wish to control.
//!
//! ## Default Controls
//!
//! This controller has a simple 6-axis control scheme, and mouse controls for camera orientation. There are also
//! bindings for capturing the mouse, both while holding the button and toggle, a run feature that increases the
//! max speed, and scrolling changes the movement speed. All keybinds can be changed by editing the [`FreeCam`]
//! component.
//!
//! | Default Key Binding | Action                 |
//! |:--------------------|:-----------------------|
//! | Mouse               | Look around            |
//! | Left click          | Capture mouse (hold)   |
//! | M                   | Capture mouse (toggle) |
//! | WASD                | Horizontal movement    |
//! | QE                  | Vertical movement      |
//! | Left shift          | Run                    |
//! | Scroll wheel        | Change movement speed  |
//!
//! The movement speed, sensitivity and friction can also be changed by the [`FreeCam`] component.
//!
//! ## Example controls
//!
//! This example also provides a few extra keybinds to change the camera sensitivity and friction.
//!
//! | Key Binding | Action               |
//! |:------------|:---------------------|
//! | Z           | Decrease sensitivity |
//! | X           | Increase snsitivity  |
//! | C           | Decrease friction    |
//! | V           | Increase friction    |

use std::f32::consts::{FRAC_PI_4, PI};

use bevy::{
    camera_controller::free_cam::{FreeCam, FreeCamPlugin},
    color::palettes::tailwind,
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Plugin that enables freecam functionality
        .add_plugins(FreeCamPlugin)
        // Example code plugins
        .add_plugins((CameraPlugin, CameraSettingsPlugin, ScenePlugin))
        .run();
}

// Plugin that spawns the camera.
struct CameraPlugin;
impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_camera);
    }
}

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 1.0, 0.0).looking_to(Vec3::X, Vec3::Y),
        // This component stores all camera settings and state, which is used by the FreeCamPlugin to
        // control it. These properties can be changed at runtime, but beware the controller system is
        // constantly using and modifying those values unless the enabled field is false.
        FreeCam {
            sensitivity: 0.2,
            friction: 25.0,
            walk_speed: 3.0,
            run_speed: 9.0,
            ..default()
        },
    ));
}

// Plugin that handles camera settings controls and information text
struct CameraSettingsPlugin;
impl Plugin for CameraSettingsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, spawn_text)
            .add_systems(Update, (update_camera_settings, update_text));
    }
}

#[derive(Component)]
struct InfoText;

fn spawn_text(mut commands: Commands) {
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(12),
            left: px(12),
            ..default()
        },
        children![Text::new(concat![
            "Z/X: decrease/increase sensitivity\n",
            "C/V: decrease/increase friction",
        ]),],
    ));

    // Mutable text marked with component
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        children![(InfoText, Text::new(""),)],
    ));
}

fn update_camera_settings(mut camera_query: Query<&mut FreeCam>, input: Res<ButtonInput<KeyCode>>) {
    let mut free_cam = camera_query.single_mut().unwrap();

    if input.pressed(KeyCode::KeyZ) {
        free_cam.sensitivity = (free_cam.sensitivity - 0.005).max(0.005);
    }
    if input.pressed(KeyCode::KeyX) {
        free_cam.sensitivity += 0.005;
    }
    if input.pressed(KeyCode::KeyC) {
        free_cam.friction = (free_cam.friction - 0.2).max(0.0);
    }
    if input.pressed(KeyCode::KeyV) {
        free_cam.friction += 0.2;
    }
}

fn update_text(mut text_query: Query<&mut Text, With<InfoText>>, camera_query: Query<&FreeCam>) {
    let mut text = text_query.single_mut().unwrap();

    let free_cam = camera_query.single().unwrap();

    text.0 = format!(
        "Sensitivity: {:.03}\nFriction: {:.01}\nSpeed: {:.02}",
        free_cam.sensitivity,
        free_cam.friction,
        free_cam.velocity.length(),
    );
}

// Plugin that spawns the scene and lighting.
struct ScenePlugin;
impl Plugin for ScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (spawn_lights, spawn_world));
    }
}

fn spawn_lights(mut commands: Commands) {
    // Main light
    commands.spawn((
        PointLight {
            color: Color::from(tailwind::ORANGE_300),
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));
    // Light behind wall
    commands.spawn((
        PointLight {
            color: Color::WHITE,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-3.5, 3.0, 0.0),
    ));
    // Light under floor
    commands.spawn((
        PointLight {
            color: Color::from(tailwind::RED_300),
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
    let cube = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let floor = meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(10.0)));
    let sphere = meshes.add(Sphere::new(0.5));
    let wall = meshes.add(Cuboid::new(0.2, 4.0, 3.0));

    let blue_material = materials.add(Color::from(tailwind::BLUE_700));
    let red_material = materials.add(Color::from(tailwind::RED_950));
    let white_material = materials.add(Color::WHITE);

    // Top side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(white_material.clone()),
    ));
    // Under side of floor
    commands.spawn((
        Mesh3d(floor.clone()),
        MeshMaterial3d(white_material.clone()),
        Transform::from_xyz(0.0, -0.01, 0.0).with_rotation(Quat::from_rotation_x(PI)),
    ));
    // Blue sphere
    commands.spawn((
        Mesh3d(sphere.clone()),
        MeshMaterial3d(blue_material.clone()),
        Transform::from_xyz(3.0, 1.5, 0.0),
    ));
    // Tall wall
    commands.spawn((
        Mesh3d(wall.clone()),
        MeshMaterial3d(white_material.clone()),
        Transform::from_xyz(-3.0, 2.0, 0.0),
    ));
    // Cube behind wall
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(blue_material.clone()),
        Transform::from_xyz(-4.2, 0.5, 0.0),
    ));
    // Hidden cube under floor
    commands.spawn((
        Mesh3d(cube.clone()),
        MeshMaterial3d(red_material.clone()),
        Transform {
            translation: Vec3::new(3.0, -2.0, 0.0),
            rotation: Quat::from_euler(EulerRot::YXZEx, FRAC_PI_4, FRAC_PI_4, 0.0),
            ..default()
        },
    ));
}
