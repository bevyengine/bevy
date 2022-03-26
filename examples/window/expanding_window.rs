use bevy::prelude::*;

const MAX_WIDTH: f32 = 400.;
const MAX_HEIGHT: f32 = 400.;

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            width: MAX_WIDTH,
            height: MAX_HEIGHT,
            scale_factor_override: Some(1.),
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .insert_resource(Phase::ContractingY)
        .add_system(change_window_size)
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
    mut windows: ResMut<Windows>,
    mut phase: ResMut<Phase>,
    mut first_complete: Local<bool>,
) {
    // Put off rendering for one frame, as currently for a frame where
    // resizing happens, nothing is presented.
    if !*first_complete {
        *first_complete = true;
        return;
    }
    let primary = windows.get_primary_mut().unwrap();
    let height = primary.height();
    let width = primary.width();
    match *phase {
        Phase::ContractingY => {
            if height <= 0.5 {
                *phase = ContractingX;
            }
            primary.set_resolution(width, (height - 4.).max(0.0))
        }
        Phase::ContractingX => {
            if width <= 0.5 {
                *phase = ExpandingY;
            }
            primary.set_resolution((width - 4.).max(0.0), height)
        }
        Phase::ExpandingY => {
            if height >= MAX_HEIGHT {
                *phase = ExpandingX;
            }
            primary.set_resolution(width, height + 4.)
        }
        Phase::ExpandingX => {
            if width >= MAX_WIDTH {
                *phase = ContractingY;
            }
            primary.set_resolution(width + 4., height)
        }
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
