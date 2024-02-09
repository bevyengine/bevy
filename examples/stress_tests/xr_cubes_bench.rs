//! A cube-spawning benchmark.
//!
//! Press the arrow keys to adjust the number of cubes, then press R to spawn
//! them.

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;

#[derive(Component)]
struct Cube;

#[derive(Resource)]
struct Spawner {
    pub width: i32,
    pub height: i32,
    pub cube_size: f32,
    pub should_spawn: bool,
    pub material: Option<Handle<StandardMaterial>>,
    pub mesh: Option<Handle<Mesh>>,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(FrameTimeDiagnosticsPlugin)
        .add_plugins(LogDiagnosticsPlugin::default())
        .add_systems(Startup, setup)
        .insert_resource(Spawner {
            should_spawn: true,
            width: 20,
            height: 100,
            cube_size: 0.01,
            mesh: None,
            material: None,
        })
        .add_systems(Update, (modify_spawn_parameters, spawn_boxes_system))
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(5.0)),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.1 })),
        material: materials.add(Color::rgb(0.8, 0.0, 0.0)),
        transform: Transform::from_xyz(0.0, 0.5, 1.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn modify_spawn_parameters(
    mut commands: Commands,
    things_query: Query<Entity, With<Cube>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut spawner: ResMut<Spawner>,
) {
    if keys.just_released(KeyCode::ArrowUp) {
        spawner.height += 10;
        println!("Boxes: {}", spawner.width * spawner.height);
    }

    if keys.just_released(KeyCode::ArrowDown) {
        spawner.height -= 10;
        println!("Boxes: {}", spawner.width * spawner.height);
    }

    if keys.just_released(KeyCode::ArrowRight) {
        spawner.width += 10;
        println!("Boxes: {}", spawner.width * spawner.height);
    }

    if keys.just_released(KeyCode::ArrowLeft) {
        spawner.width -= 10;
        println!("Boxes: {}", spawner.width * spawner.height);
    }

    if keys.just_released(KeyCode::KeyR) {
        for e in things_query.iter() {
            commands.entity(e).despawn()
        }
        spawner.should_spawn = true;
        println!("Spawning started");
    }
}

fn spawn_boxes_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    mut spawner: ResMut<Spawner>,
) {
    if !spawner.should_spawn {
        return;
    }
    let cube_size = spawner.cube_size;

    if spawner.mesh.is_none() {
        spawner.mesh = Some(meshes.add(Mesh::from(shape::Cube { size: cube_size })));
        spawner.material = Some(material_assets.add(Color::GOLD));
    }

    println!("We are spawning cubes");
    spawner.should_spawn = false;
    let mesh = spawner.mesh.as_ref().unwrap();
    let material = spawner.material.as_ref().unwrap();
    let cube_on_x = spawner.width;
    let cube_on_y = spawner.height;
    let margin = cube_size * 0.1 as f32;
    let offset = Vec3::new(-cube_on_x as f32 * 0.5 * cube_size, 0.0, -3.0);

    for x in 0..cube_on_x {
        for y in 0..cube_on_y {
            commands.spawn((
                PbrBundle {
                    mesh: mesh.clone_weak(),
                    material: material.clone_weak(),
                    transform: Transform::from_xyz(
                        x as f32 * (cube_size + margin) + offset.x,
                        y as f32 * (cube_size + margin) + offset.y,
                        offset.z,
                    ),
                    ..default()
                },
                Cube,
            ));
        }
    }
}
