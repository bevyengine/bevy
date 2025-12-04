//! Interactive example demonstrating the full Earthworks plugin with direct machine control.
//!
//! Controls:
//! - Mouse drag: Orbit camera
//! - Scroll wheel: Zoom in/out
//! - WASD: Move bulldozer (forward/back/turn)
//! - Q/E: Lower/raise blade
//! - Space: Quick stop
//! - F: Follow cam toggle
//! - G: Toggle work envelope gizmos
//! - 1-4: Set playback speed (0.5x, 1x, 2x, 4x)

use bevy::core_pipeline::tonemapping::Tonemapping;
use bevy::light::{CascadeShadowConfigBuilder, GlobalAmbientLight};
use bevy::pbr::{DistanceFog, FogFalloff};
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;
use bevy_earthworks::camera::OrbitCamera;
use bevy_earthworks::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Earthworks - Bulldozer Simulation".to_string(),
                resolution: (1600u32, 900u32).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EarthworksPlugin::default())
        .add_plugins(DirectControlPlugin)
        // Warm hazy sky - matches fog for seamless horizon
        .insert_resource(ClearColor(Color::srgb(0.85, 0.82, 0.75)))
        .insert_resource(FollowCam { enabled: true })
        .add_systems(Startup, setup)
        .add_systems(Startup, spawn_hud)
        .add_systems(Update, (keyboard_controls, follow_camera, update_hud))
        .run();
}

#[derive(Resource)]
struct FollowCam {
    enabled: bool,
}

fn setup(
    mut commands: Commands,
    mut terrain: ResMut<VoxelTerrain>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn orbit camera with enhanced post-processing
    // Camera positioned to see the central worksite with terrain extending to edges
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(80.0, 50.0, 80.0).looking_at(Vec3::new(48.0, 0.0, 48.0), Vec3::Y),
        OrbitCamera::new()
            .with_target(Vec3::new(48.0, 4.0, 48.0))
            .with_distance(70.0),
        // Tonemapping for better color reproduction
        Tonemapping::TonyMcMapface,
        // Subtle bloom for sun and highlights (enables HDR automatically)
        Bloom {
            intensity: 0.15,
            ..Bloom::NATURAL
        },
        // Distance fog for atmosphere and depth - warm dusty haze
        DistanceFog {
            color: Color::srgba(0.85, 0.82, 0.75, 1.0), // Warm haze, matches ClearColor
            directional_light_color: Color::srgba(1.0, 0.9, 0.7, 0.6), // Golden sun glow
            directional_light_exponent: 20.0,
            falloff: FogFalloff::Exponential { density: 0.012 },
        },
    ));

    // Main sun light - warm golden hour feel
    commands.spawn((
        DirectionalLight {
            illuminance: 80000.0, // Brighter for HDR
            shadows_enabled: true,
            color: Color::srgb(1.0, 0.92, 0.8), // Warm sunlight
            ..default()
        },
        Transform::from_xyz(50.0, 80.0, 30.0).looking_at(Vec3::ZERO, Vec3::Y),
        // Better shadow cascades for larger terrain
        CascadeShadowConfigBuilder {
            num_cascades: 4,
            first_cascade_far_bound: 10.0,
            maximum_distance: 150.0,
            ..default()
        }
        .build(),
    ));

    // Sky fill light - subtle blue from above
    commands.spawn((
        DirectionalLight {
            illuminance: 8000.0,
            shadows_enabled: false,
            color: Color::srgb(0.7, 0.8, 1.0), // Cool sky blue
            ..default()
        },
        Transform::from_xyz(0.0, 100.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Ambient light - warm ground bounce
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.95, 0.9, 0.85),
        brightness: 400.0,
        affects_lightmapped_meshes: true,
    });

    // Create larger terrain with interesting features
    use bevy_earthworks::terrain::{Chunk, ChunkCoord, DirtyChunk, MaterialId, Voxel};

    // 12x12 chunk terrain - much larger world extending beyond worksite
    // This gives us ~192m x 192m of terrain
    let terrain_chunks = 12;
    let worksite_center = (terrain_chunks * 16) / 2; // Center of terrain in voxels

    for cx in 0..terrain_chunks {
        for cz in 0..terrain_chunks {
            let mut chunk = Chunk::new();

            for x in 0..16 {
                for z in 0..16 {
                    let world_x = cx * 16 + x as i32;
                    let world_z = cz * 16 + z as i32;

                    // Distance from worksite center (for height variation)
                    let dx = world_x - worksite_center;
                    let dz = world_z - worksite_center;
                    let dist_from_center = ((dx * dx + dz * dz) as f32).sqrt();

                    // Calculate terrain height - worksite in center, terrain rises at edges
                    let base_height = calculate_terrain_height(world_x as f32, world_z as f32);

                    // Add gentle hills at the edges of the map
                    let edge_height = if dist_from_center > 40.0 {
                        ((dist_from_center - 40.0) * 0.08).min(4.0) as usize
                    } else {
                        0
                    };

                    let height = base_height + edge_height;

                    // Fill ground layers with varied materials
                    for y in 0..height.min(16) {
                        let material = if y < 2 {
                            MaterialId::Rock
                        } else if y < height - 2 {
                            // Vary subsurface materials
                            if (world_x + world_z + y as i32) % 7 == 0 {
                                MaterialId::Clay
                            } else if (world_x * 3 + world_z * 2 + y as i32) % 11 == 0 {
                                MaterialId::Gravel
                            } else {
                                MaterialId::Dirt
                            }
                        } else if y == height - 1 {
                            MaterialId::Topsoil
                        } else {
                            MaterialId::Dirt
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

    // Spawn player-controlled bulldozer in the center worksite area
    let center_world = worksite_center as f32 * 0.5; // Convert to world units
    spawn_bulldozer(&mut commands, &mut meshes, &mut materials, Vec3::new(center_world - 12.0, 8.0, center_world - 12.0));

    // Spawn a parked excavator nearby
    spawn_excavator(&mut commands, &mut meshes, &mut materials, Vec3::new(center_world + 20.0, 6.0, center_world + 20.0));

    // Create initial job - level the central mound area (adjusted for larger terrain)
    use bevy_math::IVec3;
    let job_center = worksite_center; // Job is at terrain center
    commands.insert_resource(CurrentJob {
        active: Some(Job::level_area(
            "Clear the Mound",
            IVec3::new(job_center - 10, 0, job_center - 10), // min corner
            IVec3::new(job_center + 10, 0, job_center + 10), // max corner
            8,                      // target height (voxels)
            500,                    // reward Zyns
        )),
        progress: 0.0,
    });

    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║        EARTHWORKS - Bulldozer Simulation                 ║");
    println!("╠═══════════════════════════════════════════════════════════╣");
    println!("║  CONTROLS:                                                ║");
    println!("║    W/S         - Drive forward/reverse                    ║");
    println!("║    A/D         - Turn left/right                          ║");
    println!("║    E           - Lower blade (dig)                        ║");
    println!("║    Q           - Raise blade                              ║");
    println!("║    Space       - Quick stop                               ║");
    println!("║    F           - Toggle follow camera                     ║");
    println!("║                                                           ║");
    println!("║  CAMERA:                                                  ║");
    println!("║    Mouse drag  - Orbit camera                             ║");
    println!("║    Scroll      - Zoom in/out                              ║");
    println!("║                                                           ║");
    println!("║  HOW TO DIG:                                              ║");
    println!("║    1. Lower blade with E                                  ║");
    println!("║    2. Drive forward with W                                ║");
    println!("║    3. Watch material accumulate                           ║");
    println!("║    4. Stop or raise blade to deposit                      ║");
    println!("║                                                           ║");
    println!("║  JOB: Clear the Mound - Level the center area to earn Zyns!║");
    println!("╚═══════════════════════════════════════════════════════════╝");
}

/// Calculate terrain height using pseudo-noise (deterministic)
fn calculate_terrain_height(x: f32, z: f32) -> usize {
    // Base terrain - gentle rolling hills
    let base = 6.0;

    // Large-scale hills
    let hill1 = 4.0 * (0.05 * x).sin() * (0.05 * z).cos();
    let hill2 = 3.0 * (0.08 * x + 1.0).cos() * (0.06 * z + 0.5).sin();

    // Medium features
    let ridge = 2.5 * ((0.12 * x).sin() * (0.1 * z).sin()).max(0.0);

    // Small variations
    let detail = 1.0 * (0.3 * x).sin() * (0.25 * z).cos();

    // Central mound for the work site
    let cx = 40.0;
    let cz = 40.0;
    let dist_center = ((x - cx).powi(2) + (z - cz).powi(2)).sqrt();
    let mound = if dist_center < 20.0 {
        5.0 * (1.0 - dist_center / 20.0).powi(2)
    } else {
        0.0
    };

    // Combine and clamp
    let height = base + hill1 + hill2 + ridge + detail + mound;
    height.max(3.0).min(14.0) as usize
}

/// Spawn a detailed bulldozer
fn spawn_bulldozer(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) {
    // Colors - CAT yellow theme
    let yellow = Color::srgb(0.95, 0.75, 0.1);
    let dark_yellow = Color::srgb(0.8, 0.6, 0.05);
    let black = Color::srgb(0.1, 0.1, 0.1);
    let dark_gray = Color::srgb(0.2, 0.2, 0.2);
    let glass = Color::srgba(0.3, 0.4, 0.5, 0.6);

    // Materials
    let body_mat = materials.add(StandardMaterial {
        base_color: yellow,
        metallic: 0.2,
        perceptual_roughness: 0.7,
        ..default()
    });
    let accent_mat = materials.add(StandardMaterial {
        base_color: dark_yellow,
        metallic: 0.3,
        perceptual_roughness: 0.6,
        ..default()
    });
    let track_mat = materials.add(StandardMaterial {
        base_color: black,
        metallic: 0.1,
        perceptual_roughness: 0.9,
        ..default()
    });
    let metal_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.8,
        perceptual_roughness: 0.3,
        ..default()
    });
    let glass_mat = materials.add(StandardMaterial {
        base_color: glass,
        metallic: 0.0,
        perceptual_roughness: 0.1,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let blade_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.9,
        perceptual_roughness: 0.2,
        ..default()
    });

    // Meshes
    let track_mesh = meshes.add(Cuboid::new(1.0, 0.8, 4.0));
    let body_mesh = meshes.add(Cuboid::new(2.4, 1.2, 3.2));
    let cab_mesh = meshes.add(Cuboid::new(1.8, 1.4, 1.6));
    let roof_mesh = meshes.add(Cuboid::new(2.0, 0.15, 1.8));
    let engine_mesh = meshes.add(Cuboid::new(2.2, 0.8, 1.2));
    let exhaust_mesh = meshes.add(Cylinder::new(0.08, 0.6));
    let blade_mesh = meshes.add(Cuboid::new(4.0, 1.2, 0.25));
    let blade_edge_mesh = meshes.add(Cuboid::new(4.2, 0.15, 0.3));
    let arm_mesh = meshes.add(Cuboid::new(0.15, 0.15, 1.5));
    let ripper_mesh = meshes.add(Cuboid::new(0.8, 0.4, 0.15));
    let ripper_tooth_mesh = meshes.add(Cuboid::new(0.1, 0.6, 0.1));

    commands
        .spawn((
            Transform::from_translation(position),
            Visibility::default(),
            Machine {
                id: "dozer-1".to_string(),
                machine_type: MachineType::Dozer,
                capacity: 8.0,
                current_load: 0.0,
                fuel: 1.0,
            },
            WorkEnvelope::Rectangular {
                width: 4.0,
                depth: 5.0,
                height: 1.5,
            },
            Mobility {
                max_speed: 6.0,
                turn_rate: 1.2,
                can_reverse: true,
                tracked: true,
            },
            MachineActivity::Idle,
            PlayerControlled,
            BladeState {
                height: 0.0,
                load: 0.0,
                capacity: 8.0,
            },
            Name::new("Bulldozer"),
        ))
        .with_children(|parent| {
            // Left track
            parent.spawn((
                Mesh3d(track_mesh.clone()),
                MeshMaterial3d(track_mat.clone()),
                Transform::from_xyz(-1.0, 0.0, 0.0),
            ));

            // Right track
            parent.spawn((
                Mesh3d(track_mesh.clone()),
                MeshMaterial3d(track_mat.clone()),
                Transform::from_xyz(1.0, 0.0, 0.0),
            ));

            // Main body
            parent.spawn((
                Mesh3d(body_mesh),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(0.0, 0.9, 0.2),
            ));

            // Cab
            parent.spawn((
                Mesh3d(cab_mesh),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(0.0, 2.0, 0.5),
            ));

            // Cab roof
            parent.spawn((
                Mesh3d(roof_mesh),
                MeshMaterial3d(accent_mat.clone()),
                Transform::from_xyz(0.0, 2.8, 0.5),
            ));

            // Cab windows (front)
            parent.spawn((
                Mesh3d(meshes.add(Cuboid::new(1.6, 0.8, 0.05))),
                MeshMaterial3d(glass_mat.clone()),
                Transform::from_xyz(0.0, 2.1, -0.35),
            ));

            // Engine hood
            parent.spawn((
                Mesh3d(engine_mesh),
                MeshMaterial3d(accent_mat.clone()),
                Transform::from_xyz(0.0, 1.0, -1.2),
            ));

            // Exhaust stack
            parent.spawn((
                Mesh3d(exhaust_mesh),
                MeshMaterial3d(metal_mat.clone()),
                Transform::from_xyz(0.8, 1.8, -0.8),
            ));

            // Blade assembly
            parent.spawn((
                Transform::from_xyz(0.0, 0.3, -2.8),
                Visibility::default(),
                BladeVisual,
                Name::new("BladeAssembly"),
            )).with_children(|blade_parent| {
                // Main blade
                blade_parent.spawn((
                    Mesh3d(blade_mesh),
                    MeshMaterial3d(blade_mat.clone()),
                    Transform::from_xyz(0.0, 0.0, 0.0),
                ));

                // Cutting edge
                blade_parent.spawn((
                    Mesh3d(blade_edge_mesh),
                    MeshMaterial3d(metal_mat.clone()),
                    Transform::from_xyz(0.0, -0.6, 0.0),
                ));

                // Left push arm
                blade_parent.spawn((
                    Mesh3d(arm_mesh.clone()),
                    MeshMaterial3d(body_mat.clone()),
                    Transform::from_xyz(-1.5, 0.3, 0.8),
                ));

                // Right push arm
                blade_parent.spawn((
                    Mesh3d(arm_mesh.clone()),
                    MeshMaterial3d(body_mat.clone()),
                    Transform::from_xyz(1.5, 0.3, 0.8),
                ));
            });

            // Rear ripper
            parent.spawn((
                Mesh3d(ripper_mesh),
                MeshMaterial3d(metal_mat.clone()),
                Transform::from_xyz(0.0, 0.4, 2.5),
            ));

            // Ripper teeth
            for i in -1..=1 {
                parent.spawn((
                    Mesh3d(ripper_tooth_mesh.clone()),
                    MeshMaterial3d(metal_mat.clone()),
                    Transform::from_xyz(i as f32 * 0.25, 0.0, 2.5),
                ));
            }
        });
}

/// Spawn a parked excavator for visual interest
fn spawn_excavator(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    position: Vec3,
) {
    let yellow = Color::srgb(0.95, 0.75, 0.1);
    let black = Color::srgb(0.1, 0.1, 0.1);
    let dark_gray = Color::srgb(0.25, 0.25, 0.25);

    let body_mat = materials.add(StandardMaterial {
        base_color: yellow,
        metallic: 0.2,
        perceptual_roughness: 0.7,
        ..default()
    });
    let track_mat = materials.add(StandardMaterial {
        base_color: black,
        metallic: 0.1,
        perceptual_roughness: 0.9,
        ..default()
    });
    let metal_mat = materials.add(StandardMaterial {
        base_color: dark_gray,
        metallic: 0.7,
        perceptual_roughness: 0.4,
        ..default()
    });

    commands
        .spawn((
            Transform::from_translation(position).with_rotation(Quat::from_rotation_y(0.8)),
            Visibility::default(),
            Machine {
                id: "excavator-1".to_string(),
                machine_type: MachineType::Excavator,
                capacity: 12.0,
                current_load: 0.0,
                fuel: 1.0,
            },
            WorkEnvelope::Toroidal {
                inner_radius: 3.0,
                outer_radius: 10.0,
                min_height: -4.0,
                max_height: 6.0,
            },
            Mobility::default(),
            MachineActivity::Idle,
            Name::new("Excavator"),
        ))
        .with_children(|parent| {
            // Track assembly (wider than dozer)
            let track_mesh = meshes.add(Cuboid::new(1.2, 0.9, 4.5));
            parent.spawn((
                Mesh3d(track_mesh.clone()),
                MeshMaterial3d(track_mat.clone()),
                Transform::from_xyz(-1.5, 0.0, 0.0),
            ));
            parent.spawn((
                Mesh3d(track_mesh),
                MeshMaterial3d(track_mat),
                Transform::from_xyz(1.5, 0.0, 0.0),
            ));

            // Upper body (rotates)
            let body_mesh = meshes.add(Cuboid::new(2.8, 1.5, 3.5));
            parent.spawn((
                Mesh3d(body_mesh),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(0.0, 1.2, 0.0),
            ));

            // Cab
            let cab_mesh = meshes.add(Cuboid::new(1.8, 1.6, 1.5));
            parent.spawn((
                Mesh3d(cab_mesh),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(-0.3, 2.5, -0.8),
            ));

            // Boom (main arm)
            let boom_mesh = meshes.add(Cuboid::new(0.5, 0.6, 4.0));
            parent.spawn((
                Mesh3d(boom_mesh),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(0.0, 2.5, -3.5)
                    .with_rotation(Quat::from_rotation_x(-0.4)),
            ));

            // Stick (second arm segment)
            let stick_mesh = meshes.add(Cuboid::new(0.4, 0.5, 3.0));
            parent.spawn((
                Mesh3d(stick_mesh),
                MeshMaterial3d(body_mat.clone()),
                Transform::from_xyz(0.0, 1.5, -6.5)
                    .with_rotation(Quat::from_rotation_x(0.6)),
            ));

            // Bucket
            let bucket_mesh = meshes.add(Cuboid::new(1.2, 0.8, 0.8));
            parent.spawn((
                Mesh3d(bucket_mesh),
                MeshMaterial3d(metal_mat),
                Transform::from_xyz(0.0, -0.5, -7.5),
            ));

            // Counterweight
            let counterweight_mesh = meshes.add(Cuboid::new(2.4, 1.0, 1.2));
            parent.spawn((
                Mesh3d(counterweight_mesh),
                MeshMaterial3d(body_mat),
                Transform::from_xyz(0.0, 1.5, 2.0),
            ));
        });
}

/// Follow camera system
fn follow_camera(
    follow_cam: Res<FollowCam>,
    dozer_query: Query<&Transform, (With<PlayerControlled>, Without<OrbitCamera>)>,
    mut camera_query: Query<&mut OrbitCamera>,
) {
    if !follow_cam.enabled {
        return;
    }

    let Ok(dozer_transform) = dozer_query.single() else {
        return;
    };

    for mut orbit in camera_query.iter_mut() {
        // Smoothly follow the dozer
        let target = dozer_transform.translation + Vec3::new(0.0, 2.0, 0.0);
        orbit.target = orbit.target.lerp(target, 0.05);
    }
}

/// HUD component markers
#[derive(Component)]
struct HudRoot;

#[derive(Component)]
struct HudBladeStatus;

#[derive(Component)]
struct HudLoadBar;

#[derive(Component)]
struct HudLoadText;

#[derive(Component)]
struct HudPosition;

#[derive(Component)]
struct HudControls;

/// Spawn the on-screen HUD
fn spawn_hud(mut commands: Commands) {
    // Main HUD container - bottom left
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                bottom: Val::Px(100.0), // Above timeline
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(15.0)),
                row_gap: Val::Px(8.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            HudRoot,
        ))
        .with_children(|parent| {
            // Title
            parent.spawn((
                Text::new("BULLDOZER STATUS"),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.3)),
            ));

            // Blade status row
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(10.0),
                    ..default()
                })
                .with_children(|row| {
                    row.spawn((
                        Text::new("Blade: "),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                    ));
                    row.spawn((
                        Text::new("TRAVELING"),
                        TextFont {
                            font_size: 16.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.3, 1.0, 0.3)),
                        HudBladeStatus,
                    ));
                });

            // Load bar container
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|col| {
                    // Load label
                    col.spawn((
                        Text::new("Load: 0.0 / 8.0 m³"),
                        TextFont {
                            font_size: 14.0,
                            ..default()
                        },
                        TextColor(Color::srgb(0.8, 0.8, 0.8)),
                        HudLoadText,
                    ));

                    // Load bar background
                    col.spawn((
                        Node {
                            width: Val::Px(200.0),
                            height: Val::Px(16.0),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.2, 0.2, 0.2)),
                    ))
                    .with_children(|bar_bg| {
                        // Load bar fill
                        bar_bg.spawn((
                            Node {
                                width: Val::Percent(0.0),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.8, 0.5, 0.2)),
                            HudLoadBar,
                        ));
                    });
                });

            // Position
            parent.spawn((
                Text::new("Position: (0.0, 0.0)"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(0.6, 0.6, 0.6)),
                HudPosition,
            ));
        });

    // Controls hint - top left
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(20.0),
                top: Val::Px(20.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(12.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            HudControls,
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("CONTROLS"),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.9, 0.3)),
            ));
            for line in [
                "W/S - Drive",
                "A/D - Turn",
                "Q/E - Raise/Lower blade",
                "F - Toggle follow cam",
            ] {
                parent.spawn((
                    Text::new(line),
                    TextFont {
                        font_size: 12.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.7, 0.7, 0.7)),
                ));
            }
        });
}

/// Update HUD with current machine state
fn update_hud(
    blade_query: Query<(&BladeState, &Transform), With<PlayerControlled>>,
    mut blade_status: Query<(&mut Text, &mut TextColor), With<HudBladeStatus>>,
    mut load_bar: Query<(&mut Node, &mut BackgroundColor), With<HudLoadBar>>,
    mut load_text: Query<&mut Text, (With<HudLoadText>, Without<HudBladeStatus>)>,
    mut position_text: Query<&mut Text, (With<HudPosition>, Without<HudBladeStatus>, Without<HudLoadText>)>,
) {
    let Ok((blade, transform)) = blade_query.single() else {
        return;
    };

    // Update blade status
    let (status_text, status_color) = if blade.height < -0.3 {
        ("DIGGING", Color::srgb(1.0, 0.4, 0.2))
    } else if blade.height < 0.0 {
        ("SCRAPING", Color::srgb(1.0, 0.7, 0.2))
    } else if blade.height < 0.5 {
        ("TRAVELING", Color::srgb(0.3, 1.0, 0.3))
    } else {
        ("RAISED", Color::srgb(0.5, 0.8, 1.0))
    };

    if let Ok((mut text, mut color)) = blade_status.single_mut() {
        **text = status_text.to_string();
        *color = TextColor(status_color);
    }

    // Update load bar
    let load_percent = (blade.load / blade.capacity * 100.0).min(100.0);
    if let Ok((mut node, mut bg_color)) = load_bar.single_mut() {
        node.width = Val::Percent(load_percent);

        // Color based on load level
        let color = if load_percent > 80.0 {
            Color::srgb(0.9, 0.3, 0.2) // Red when nearly full
        } else if load_percent > 50.0 {
            Color::srgb(0.9, 0.7, 0.2) // Orange when half full
        } else {
            Color::srgb(0.6, 0.4, 0.2) // Brown normally
        };
        *bg_color = BackgroundColor(color);
    }

    // Update load text
    if let Ok(mut text) = load_text.single_mut() {
        **text = format!("Load: {:.1} / {:.1} m³", blade.load, blade.capacity);
    }

    // Update position
    if let Ok(mut text) = position_text.single_mut() {
        **text = format!(
            "Position: ({:.1}, {:.1})",
            transform.translation.x, transform.translation.z
        );
    }
}

/// Keyboard controls for playback and visualization
fn keyboard_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut playback: ResMut<PlanPlayback>,
    mut follow_cam: ResMut<FollowCam>,
) {
    // F - toggle follow camera
    if keyboard.just_pressed(KeyCode::KeyF) {
        follow_cam.enabled = !follow_cam.enabled;
        println!("Follow camera: {}", if follow_cam.enabled { "ON" } else { "OFF" });
    }

    // P - toggle play/pause
    if keyboard.just_pressed(KeyCode::KeyP) {
        playback.toggle();
        let state = if playback.is_playing() { "Playing" } else { "Paused" };
        println!("Playback: {}", state);
    }

    // R - reset
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
