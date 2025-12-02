//! Interactive example demonstrating the full Earthworks plugin.
//!
//! Controls:
//! - Mouse drag: Orbit camera
//! - Scroll wheel: Zoom in/out
//! - Space: Toggle play/pause
//! - R: Reset playback to start
//! - 1-4: Set playback speed (0.5x, 1x, 2x, 4x)

use bevy::prelude::*;
use bevy_earthworks::camera::OrbitCamera;
use bevy_earthworks::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Earthworks - Interactive Demo".to_string(),
                resolution: (1280u32, 720u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EarthworksPlugin::default())
        .add_systems(Startup, setup)
        .add_systems(Update, keyboard_controls)
        .run();
}

fn setup(
    mut commands: Commands,
    mut terrain: ResMut<VoxelTerrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn orbit camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(40.0, 30.0, 40.0).looking_at(Vec3::new(8.0, 0.0, 8.0), Vec3::Y),
        OrbitCamera::new()
            .with_target(Vec3::new(24.0, 4.0, 24.0))
            .with_distance(50.0),
    ));

    // Spawn directional light (sun)
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(50.0, 100.0, 50.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light (spawned as entity in Bevy 0.18)
    commands.spawn(AmbientLight {
        color: Color::srgb(0.9, 0.95, 1.0),
        brightness: 500.0,
        affects_lightmapped_meshes: true,
    });

    // Create terrain with varied elevation
    use bevy_earthworks::terrain::{Chunk, ChunkCoord, DirtyChunk, MaterialId, Voxel};

    for cx in 0..3 {
        for cz in 0..3 {
            let mut chunk = Chunk::new();

            for x in 0..16 {
                for z in 0..16 {
                    // Create varied terrain - a hill in the middle
                    let world_x = cx * 16 + x as i32;
                    let world_z = cz * 16 + z as i32;

                    // Distance from center (24, 24)
                    let dx = (world_x - 24) as f32;
                    let dz = (world_z - 24) as f32;
                    let dist = (dx * dx + dz * dz).sqrt();

                    // Hill height based on distance
                    let hill_height = if dist < 12.0 {
                        (8.0 * (1.0 - dist / 12.0)) as usize
                    } else {
                        0
                    };

                    // Fill ground layers
                    let base_height = 4 + hill_height;
                    for y in 0..base_height.min(16) {
                        let material = if y < 2 {
                            MaterialId::Rock
                        } else if y < base_height - 1 {
                            MaterialId::Dirt
                        } else {
                            MaterialId::Topsoil // Use Topsoil as grass layer
                        };
                        chunk.set(x, y, z, Voxel::solid(material));
                    }
                }
            }

            let coord = ChunkCoord::new(cx, 0, cz);
            let entity = commands.spawn((chunk, coord, DirtyChunk)).id();
            terrain.set_chunk_entity(coord, entity);
        }
    }

    // Spawn a simple excavator representation (placeholder mesh)
    let excavator_mesh = meshes.add(Cuboid::new(2.0, 1.5, 3.0));
    let excavator_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.6, 0.1),
        ..default()
    });

    commands.spawn((
        Mesh3d(excavator_mesh.clone()),
        MeshMaterial3d(excavator_material.clone()),
        Transform::from_xyz(5.0, 4.5, 5.0),
        Machine {
            id: "excavator-1".to_string(),
            machine_type: MachineType::Excavator,
            capacity: 10.0,
            current_load: 0.0,
            fuel: 1.0,
        },
        WorkEnvelope::Toroidal {
            inner_radius: 3.0,
            outer_radius: 8.0,
            min_height: -3.0,
            max_height: 2.0,
        },
        Mobility::default(),
        MachineActivity::Idle,
    ));

    // Spawn a dozer
    let dozer_mesh = meshes.add(Cuboid::new(2.5, 1.2, 3.5));
    let dozer_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.1),
        ..default()
    });

    commands.spawn((
        Mesh3d(dozer_mesh),
        MeshMaterial3d(dozer_material),
        Transform::from_xyz(35.0, 4.5, 35.0),
        Machine {
            id: "dozer-1".to_string(),
            machine_type: MachineType::Dozer,
            capacity: 5.0,
            current_load: 0.0,
            fuel: 1.0,
        },
        WorkEnvelope::Rectangular {
            width: 3.0,
            depth: 4.0,
            height: 1.0,
        },
        Mobility::default(),
        MachineActivity::Idle,
    ));

    println!("===========================================");
    println!("  Earthworks Interactive Demo");
    println!("===========================================");
    println!("");
    println!("Controls:");
    println!("  Mouse drag  - Orbit camera");
    println!("  Scroll      - Zoom in/out");
    println!("  Space       - Play/Pause");
    println!("  R           - Reset to start");
    println!("  1-4         - Playback speed");
    println!("");
    println!("===========================================");
}

/// Keyboard controls for playback and visualization
fn keyboard_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut playback: ResMut<PlanPlayback>,
) {
    // Space - toggle play/pause
    if keyboard.just_pressed(KeyCode::Space) {
        playback.toggle();
        let state = if playback.is_playing() { "Playing" } else { "Paused" };
        println!("Playback: {}", state);
    }

    // R - reset to start
    if keyboard.just_pressed(KeyCode::KeyR) {
        playback.reset();
        println!("Reset to start");
    }

    // Number keys for playback speed
    if keyboard.just_pressed(KeyCode::Digit1) {
        playback.set_speed(0.5);
        println!("Speed: 0.5x");
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        playback.set_speed(1.0);
        println!("Speed: 1.0x");
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        playback.set_speed(2.0);
        println!("Speed: 2.0x");
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        playback.set_speed(4.0);
        println!("Speed: 4.0x");
    }
}
