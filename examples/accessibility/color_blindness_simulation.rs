//! Small demo of how to use color blindness simulation
//! Shows a small scene, with four different cubes
//!
//! Holding the Space key enables the simulation
//! Pressing N cycles through the modes

use bevy::{
    core_pipeline::clear_color::ClearColorConfig,
    prelude::*,
    render::camera::Viewport,
    window::{close_on_esc, WindowId, WindowResized},
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // add the plugin
        .add_plugin(ColorBlindnessPlugin)
        .add_startup_system(setup)
        .add_system(close_on_esc)
        .add_system(change_mode)
        .add_system(set_camera_viewports)
        .run();
}

#[derive(Component)]
struct LeftCamera;

#[derive(Component)]
struct RightCamera;

/// set up a simple 3D scene
fn setup(
    windows: Res<Windows>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // create a small world
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    let cube = meshes.add(Mesh::from(shape::Cube { size: 0.5 }));
    commands.spawn_bundle(PbrBundle {
        mesh: cube.clone(),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    commands.spawn_bundle(PbrBundle {
        mesh: cube.clone(),
        material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
        transform: Transform::from_xyz(2.0, 0.5, 0.0),
        ..default()
    });
    commands.spawn_bundle(PbrBundle {
        mesh: cube.clone(),
        material: materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
        transform: Transform::from_xyz(3.0, 0.5, 0.0),
        ..default()
    });
    commands.spawn_bundle(PbrBundle {
        mesh: cube,
        material: materials.add(Color::rgb(0.0, 0.0, 1.0).into()),
        transform: Transform::from_xyz(4.0, 0.5, 0.0),
        ..default()
    });
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    let window = windows.primary();
    // create the cameras
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                viewport: Some(Viewport {
                    physical_position: UVec2::new(0, 0),
                    physical_size: UVec2::new(
                        window.physical_width() / 2,
                        window.physical_height(),
                    ),
                    ..default()
                }),
                ..Default::default()
            },
            ..default()
        })
        .insert(ColorBlindnessCamera {
            mode: ColorBlindnessMode::Deuteranopia,
            enabled: false,
        })
        .insert(LeftCamera);
    // create the cameras
    commands
        .spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
            camera: Camera {
                viewport: Some(Viewport {
                    physical_position: UVec2::new(window.physical_width() / 2, 0),
                    physical_size: UVec2::new(
                        window.physical_width() / 2,
                        window.physical_height(),
                    ),
                    ..default()
                }),
                priority: 1,
                ..Default::default()
            },
            camera_3d: Camera3d {
                // dont clear on the second camera because the first camera already cleared the window
                clear_color: ClearColorConfig::None,
                ..default()
            },
            ..default()
        })
        .insert(ColorBlindnessCamera {
            mode: ColorBlindnessMode::Achromatomaly,
            enabled: false,
        })
        .insert(RightCamera);
}

fn change_mode(input: Res<Input<KeyCode>>, mut cameras: Query<&mut ColorBlindnessCamera>) {
    for mut camera in &mut cameras {
        // cycle through the modes by pressing N
        if input.just_pressed(KeyCode::N) {
            camera.mode.cycle();
            println!("Changed to {:?}", camera.mode);
        }

        camera.enabled = input.pressed(KeyCode::Space);
    }
}

fn set_camera_viewports(
    windows: Res<Windows>,
    mut resize_events: EventReader<WindowResized>,
    mut left_camera: Query<&mut Camera, (With<LeftCamera>, Without<RightCamera>)>,
    mut right_camera: Query<&mut Camera, With<RightCamera>>,
) {
    // We need to dynamically resize the camera's viewports whenever the window size changes
    // so then each camera always takes up half the screen.
    // A resize_event is sent when the window is first created, allowing us to reuse this system for initial setup.
    for resize_event in resize_events.iter() {
        if resize_event.id == WindowId::primary() {
            let window = windows.primary();
            let mut left_camera = left_camera.single_mut();
            left_camera.viewport = Some(Viewport {
                physical_position: UVec2::new(0, 0),
                physical_size: UVec2::new(window.physical_width() / 2, window.physical_height()),
                ..default()
            });

            let mut right_camera = right_camera.single_mut();
            right_camera.viewport = Some(Viewport {
                physical_position: UVec2::new(window.physical_width() / 2, 0),
                physical_size: UVec2::new(window.physical_width() / 2, window.physical_height()),
                ..default()
            });
        }
    }
}
