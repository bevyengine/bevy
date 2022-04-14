use bevy::{input::system::exit_on_esc_system, prelude::*};

const MAX_WIDTH: u16 = 401;
const MAX_HEIGHT: u16 = 401;

struct Dimensions {
    width: u16,
    height: u16,
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: MAX_WIDTH.try_into().unwrap(),
            height: MAX_HEIGHT.try_into().unwrap(),
            scale_factor_override: Some(1.),
            ..Default::default()
        })
        .insert_resource(Dimensions {
            width: MAX_WIDTH,
            height: MAX_HEIGHT,
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(Phase::ContractingY)
        .add_system(change_window_size)
        .add_system(sync_dimensions)
        .add_system(exit_on_esc_system)
        .add_startup_system(setup)
        .run();
}

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
    if !*first_complete {
        *first_complete = true;
        return;
    }
    let height = windows.height;
    let width = windows.width;
    match *phase {
        Phase::ContractingY => {
            if windows.height <= 1 {
                *phase = ContractingX;
            } else {
                windows.height -= 4;
            }
        }
        Phase::ContractingX => {
            if width <= 1 {
                *phase = ExpandingY;
            } else {
                windows.width -= 4;
            }
        }
        Phase::ExpandingY => {
            if height >= MAX_HEIGHT {
                *phase = ExpandingX;
            } else {
                windows.height += 4;
            }
        }
        Phase::ExpandingX => {
            if width >= MAX_WIDTH {
                *phase = ContractingY;
            } else {
                windows.width += 4;
            }
        }
    }
}

fn sync_dimensions(dim: Res<Dimensions>, mut windows: ResMut<Windows>) {
    if dim.is_changed() {
        windows.get_primary_mut().unwrap().set_resolution(
            dim.width.try_into().unwrap(),
            dim.height.try_into().unwrap(),
        );
    }
}

/// A simple 3d scene, taken from the `3d_scene` example
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
