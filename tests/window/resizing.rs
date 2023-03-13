//! A test to confirm that `bevy` allows setting the window to arbitrary small sizes
//! This is run in CI to ensure that this doesn't regress again.

use bevy::{core_pipeline::clear_color::ClearColorConfig, prelude::*, window::WindowResolution};

// The smallest size reached is 1x1, as X11 doesn't support windows with a 0 dimension
// TODO: Add a check for platforms other than X11 for 0xk and kx0, despite those currently unsupported on CI.
const MAX_WIDTH: u16 = 401;
const MAX_HEIGHT: u16 = 401;
const MIN_WIDTH: u16 = 1;
const MIN_HEIGHT: u16 = 1;
const RESIZE_STEP: u16 = 4;

#[derive(Resource)]
struct Dimensions {
    width: u16,
    height: u16,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(MAX_WIDTH as f32, MAX_HEIGHT as f32)
                        .with_scale_factor_override(1.0),
                    title: "Resizing".into(),
                    ..default()
                }),
                ..default()
            }),
        )
        .insert_resource(Dimensions {
            width: MAX_WIDTH,
            height: MAX_HEIGHT,
        })
        .insert_resource(Phase::ContractingY)
        .add_systems(Startup, (setup_3d, setup_2d))
        .add_systems(
            Update,
            (
                change_window_size,
                sync_dimensions,
                bevy::window::close_on_esc,
            ),
        )
        .run();
}

#[derive(Resource)]
enum Phase {
    ContractingY,
    ContractingX,
    ExpandingY,
    ExpandingX,
}

use Phase::*;

fn change_window_size(
    mut windows: ResMut<Dimensions>,
    mut phase: ResMut<Phase>,
    mut first_complete: Local<bool>,
) {
    // Put off rendering for one frame, as currently for a frame where
    // resizing happens, nothing is presented.
    // TODO: Debug and fix this if feasible
    if !*first_complete {
        *first_complete = true;
        return;
    }
    let height = windows.height;
    let width = windows.width;
    match *phase {
        Phase::ContractingY => {
            if height <= MIN_HEIGHT {
                *phase = ContractingX;
            } else {
                windows.height -= RESIZE_STEP;
            }
        }
        Phase::ContractingX => {
            if width <= MIN_WIDTH {
                *phase = ExpandingY;
            } else {
                windows.width -= RESIZE_STEP;
            }
        }
        Phase::ExpandingY => {
            if height >= MAX_HEIGHT {
                *phase = ExpandingX;
            } else {
                windows.height += RESIZE_STEP;
            }
        }
        Phase::ExpandingX => {
            if width >= MAX_WIDTH {
                *phase = ContractingY;
            } else {
                windows.width += RESIZE_STEP;
            }
        }
    }
}

fn sync_dimensions(dim: Res<Dimensions>, mut windows: Query<&mut Window>) {
    if dim.is_changed() {
        let mut window = windows.single_mut();
        window.resolution.set(dim.width as f32, dim.height as f32);
    }
}

/// A simple 3d scene, taken from the `3d_scene` example
fn setup_3d(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane {
            size: 5.0,
            subdivisions: 0,
        })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

/// A simple 2d scene, taken from the `rect` example
fn setup_2d(mut commands: Commands) {
    commands.spawn(Camera2dBundle {
        camera: Camera {
            // render the 2d camera after the 3d camera
            order: 1,
            ..default()
        },
        camera_2d: Camera2d {
            // do not use a clear color
            clear_color: ClearColorConfig::None,
        },
        ..default()
    });
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 50.0)),
            ..default()
        },
        ..default()
    });
}
