//! Enhanced target application with very obvious movement and real-time updates
//!
//! This example creates entities that move continuously and update their components
//! frequently so you can see real-time updates in the remote inspector.
//!
//! Run this first, then run the inspector to see live updates:
//! 1. cargo run --example moving_target_app --features bevy_remote
//! 2. cargo run --bin bevy_remote_inspector

use bevy::prelude::*;
use std::time::Duration;
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};

/// Moving player with frequently changing stats
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Player {
    pub health: f32,
    pub speed: f32,
    pub level: u32,
    pub experience: f32,
    pub energy: f32,
}

/// Moving enemy with changing behavior
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Enemy {
    pub damage: f32,
    pub health: f32,
    pub ai_state: String,
    pub target_distance: f32,
    pub anger_level: f32,
}

/// Projectile that moves very fast
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Projectile {
    pub damage: f32,
    pub speed: f32,
    pub lifetime: f32,
}

/// Orbital movement pattern
#[derive(Component)]
struct OrbitalMover {
    pub radius: f32,
    pub speed: f32,
    pub center: Vec3,
    pub current_angle: f32,
}

/// Linear movement with boundaries
#[derive(Component)]
struct LinearMover {
    pub velocity: Vec3,
    pub bounce_bounds: f32,
}

/// Pulsing scale animation
#[derive(Component)]
struct Pulser {
    pub base_scale: f32,
    pub pulse_amplitude: f32,
    pub pulse_speed: f32,
}

fn main() {
    println!("=== ENHANCED MOVING TARGET APP FOR REMOTE INSPECTOR ===");
    println!("bevy_remote available at: http://localhost:15702");
    println!("");
    println!("What you'll see in the remote inspector:");
    println!("  • Entities moving in real-time (Transform updates)");
    println!("  • Player stats changing every frame (health, energy, etc.)");
    println!("  • Enemy AI states switching (Idle -> Hunting -> Attacking)");
    println!("  • Fast-moving projectiles with decreasing lifetime");
    println!("  • Orbital and linear movement patterns");
    println!("  • Scale pulsing animations");
    println!("  • New entities spawning every 5 seconds");
    println!("");
    println!("Start the remote inspector now to see live updates!");
    println!("cargo run --bin bevy_remote_inspector");
    println!("");
    
    App::new()
        .add_plugins(DefaultPlugins)
        
        // Enable bevy_remote for inspector connection
        .add_plugins(RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        
        // Register components for reflection
        .register_type::<Player>()
        .register_type::<Enemy>()
        .register_type::<Projectile>()
        .register_type::<Transform>()
        .register_type::<Name>()
        
        // Setup systems
        .add_systems(Startup, setup_moving_scene)
        .add_systems(Update, (
            update_player_realtime,
            update_enemy_ai,
            move_orbital_entities,
            move_linear_entities,
            update_pulsers,
            move_projectiles,
            spawn_frequent_entities,
            cleanup_old_projectiles,
        ))
        
        .run();
}

/// Set up a scene with lots of moving entities
fn setup_moving_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Setting up moving scene...");
    
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 15.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("MainCamera"),
    ));
    
    // Lighting
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 3000.0,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
        Name::new("MainLight"),
    ));
    
    // Materials
    let player_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.9, 0.2),
        metallic: 0.8,
        ..default()
    });
    let enemy_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.2, 0.2),
        metallic: 0.6,
        ..default()
    });
    let projectile_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.9, 0.9, 0.2),
        emissive: LinearRgba::rgb(0.5, 0.5, 0.0),
        ..default()
    });
    
    // Meshes
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sphere_mesh = meshes.add(Sphere::new(0.5));
    
    // Player entity with orbital movement and real-time stat updates
    commands.spawn((
        Player {
            health: 100.0,
            speed: 5.0,
            level: 1,
            experience: 0.0,
            energy: 100.0,
        },
        Mesh3d(cube_mesh.clone()),
        MeshMaterial3d(player_material),
        Transform::from_xyz(5.0, 0.0, 0.0),
        OrbitalMover {
            radius: 5.0,
            speed: 1.0,
            center: Vec3::ZERO,
            current_angle: 0.0,
        },
        Pulser {
            base_scale: 1.0,
            pulse_amplitude: 0.3,
            pulse_speed: 2.0,
        },
        Name::new("MovingPlayer"),
    ));
    
    // Multiple enemies with different movement patterns
    for i in 0..4 {
        let angle = i as f32 * std::f32::consts::PI * 0.5;
        let x = angle.cos() * 8.0;
        let z = angle.sin() * 8.0;
        
        commands.spawn((
            Enemy {
                damage: 20.0 + i as f32 * 5.0,
                health: 100.0,
                ai_state: "Idle".to_string(),
                target_distance: 0.0,
                anger_level: 0.0,
            },
            Mesh3d(cube_mesh.clone()),
            MeshMaterial3d(enemy_material.clone()),
            Transform::from_xyz(x, 0.0, z),
            LinearMover {
                velocity: Vec3::new(
                    (i as f32 * 0.7).sin() * 3.0,
                    0.0,
                    (i as f32 * 0.9).cos() * 3.0,
                ),
                bounce_bounds: 12.0,
            },
            Name::new(format!("Enemy_{}", i + 1)),
        ));
    }
    
    // Some projectiles that move very fast
    for i in 0..6 {
        let angle = i as f32 * std::f32::consts::PI / 3.0;
        let x = angle.cos() * 3.0;
        let z = angle.sin() * 3.0;
        
        commands.spawn((
            Projectile {
                damage: 50.0,
                speed: 15.0,
                lifetime: 5.0,
            },
            Mesh3d(sphere_mesh.clone()),
            MeshMaterial3d(projectile_material.clone()),
            Transform::from_xyz(x, 1.0, z),
            LinearMover {
                velocity: Vec3::new(
                    angle.cos() * 8.0,
                    (i as f32 * 0.5).sin() * 4.0,
                    angle.sin() * 8.0,
                ),
                bounce_bounds: 15.0,
            },
            Name::new(format!("Projectile_{}", i + 1)),
        ));
    }
    
    println!("Moving scene setup complete!");
    println!("Created: 1 Camera, 1 Light, 1 Player, 4 Enemies, 6 Projectiles");
}

/// Update player stats in real-time for obvious changes
fn update_player_realtime(
    time: Res<Time>,
    mut player_query: Query<&mut Player>,
) {
    for mut player in player_query.iter_mut() {
        let elapsed = time.elapsed_secs();
        
        // Health oscillates between 80-100
        player.health = 90.0 + (elapsed * 2.0).sin() * 10.0;
        
        // Speed changes rapidly
        player.speed = 5.0 + (elapsed * 3.0).sin() * 2.0;
        
        // Experience increases steadily
        player.experience += time.delta_secs() * 10.0;
        
        // Energy oscillates quickly
        player.energy = 75.0 + (elapsed * 4.0).sin() * 25.0;
        
        // Level up based on experience
        let new_level = (player.experience / 100.0) as u32 + 1;
        if new_level > player.level {
            player.level = new_level;
            println!("Player leveled up to {}!", player.level);
        }
    }
}

/// Update enemy AI states and stats
fn update_enemy_ai(
    time: Res<Time>,
    mut enemy_query: Query<&mut Enemy>,
    player_query: Query<&Transform, (With<Player>, Without<Enemy>)>,
) {
    let elapsed = time.elapsed_secs();
    
    for (i, mut enemy) in enemy_query.iter_mut().enumerate() {
        // Calculate distance to player if exists
        if let Ok(_player_transform) = player_query.single() {
            // For this example, we'll use a dummy distance calculation
            enemy.target_distance = 5.0 + (elapsed + i as f32).sin() * 3.0;
        }
        
        // Cycle through AI states
        let state_cycle = (elapsed + i as f32 * 2.0) % 12.0;
        enemy.ai_state = if state_cycle < 4.0 {
            "Idle"
        } else if state_cycle < 8.0 {
            "Hunting"
        } else {
            "Attacking"
        }.to_string();
        
        // Anger level changes based on state
        enemy.anger_level = match enemy.ai_state.as_str() {
            "Idle" => 0.0 + (elapsed * 0.5).sin() * 0.2,
            "Hunting" => 0.5 + (elapsed * 2.0).sin() * 0.3,
            "Attacking" => 0.8 + (elapsed * 4.0).sin() * 0.2,
            _ => 0.0,
        };
        
        // Health regenerates slowly
        if enemy.health < 100.0 {
            enemy.health = (enemy.health + time.delta_secs() * 5.0).min(100.0);
        }
        
        // Damage fluctuates slightly
        enemy.damage = 20.0 + i as f32 * 5.0 + (elapsed * 1.5).sin() * 2.0;
    }
}

/// Move entities in orbital patterns
fn move_orbital_entities(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut OrbitalMover)>,
) {
    for (mut transform, mut orbital) in query.iter_mut() {
        orbital.current_angle += orbital.speed * time.delta_secs();
        
        let x = orbital.center.x + orbital.radius * orbital.current_angle.cos();
        let z = orbital.center.z + orbital.radius * orbital.current_angle.sin();
        
        transform.translation.x = x;
        transform.translation.z = z;
        
        // Rotate the entity as it orbits
        transform.rotation = Quat::from_rotation_y(orbital.current_angle);
    }
}

/// Move entities with linear movement and bouncing
fn move_linear_entities(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut LinearMover)>,
) {
    for (mut transform, mut mover) in query.iter_mut() {
        transform.translation += mover.velocity * time.delta_secs();
        
        // Bounce off boundaries
        if transform.translation.x.abs() > mover.bounce_bounds {
            mover.velocity.x *= -1.0;
            transform.translation.x = transform.translation.x.signum() * mover.bounce_bounds;
        }
        if transform.translation.y.abs() > mover.bounce_bounds {
            mover.velocity.y *= -1.0;
            transform.translation.y = transform.translation.y.signum() * mover.bounce_bounds;
        }
        if transform.translation.z.abs() > mover.bounce_bounds {
            mover.velocity.z *= -1.0;
            transform.translation.z = transform.translation.z.signum() * mover.bounce_bounds;
        }
    }
}

/// Update pulsing scale animations
fn update_pulsers(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Pulser)>,
) {
    for (mut transform, pulser) in query.iter_mut() {
        let scale_factor = pulser.base_scale + 
            (time.elapsed_secs() * pulser.pulse_speed).sin() * pulser.pulse_amplitude;
        transform.scale = Vec3::splat(scale_factor);
    }
}

/// Update projectile lifetimes and remove old ones
fn move_projectiles(
    time: Res<Time>,
    mut projectile_query: Query<&mut Projectile>,
) {
    for mut projectile in projectile_query.iter_mut() {
        projectile.lifetime -= time.delta_secs();
        // Speed decreases over time
        projectile.speed = (projectile.speed - time.delta_secs() * 2.0).max(1.0);
    }
}

/// Clean up old projectiles
fn cleanup_old_projectiles(
    mut commands: Commands,
    projectile_query: Query<(Entity, &Projectile)>,
) {
    for (entity, projectile) in projectile_query.iter() {
        if projectile.lifetime <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

/// Spawn new entities frequently for dynamic testing
fn spawn_frequent_entities(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: Local<Timer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Initialize timer on first run
    if timer.duration() == Duration::ZERO {
        *timer = Timer::from_seconds(5.0, TimerMode::Repeating);
    }
    
    timer.tick(time.delta());
    
    if timer.just_finished() {
        let count = (time.elapsed_secs() / 5.0) as u32;
        let angle = count as f32 * 0.8;
        
        // Create projectile with random color
        let dynamic_material = materials.add(StandardMaterial {
            base_color: Color::srgb(
                ((count as f32 * 0.7).sin() + 1.0) * 0.5,
                ((count as f32 * 1.1).cos() + 1.0) * 0.5,
                ((count as f32 * 1.3).sin() + 1.0) * 0.5,
            ),
            emissive: LinearRgba::rgb(0.2, 0.2, 0.0),
            ..default()
        });
        
        commands.spawn((
            Projectile {
                damage: 30.0 + (count as f32 * 0.5).sin() * 10.0,
                speed: 10.0 + count as f32 * 0.5,
                lifetime: 8.0,
            },
            Mesh3d(meshes.add(Sphere::new(0.3))),
            MeshMaterial3d(dynamic_material),
            Transform::from_xyz(
                angle.cos() * 2.0,
                2.0,
                angle.sin() * 2.0,
            ),
            LinearMover {
                velocity: Vec3::new(
                    angle.cos() * 6.0,
                    (count as f32 * 0.3).sin() * 3.0,
                    angle.sin() * 6.0,
                ),
                bounce_bounds: 18.0,
            },
            Name::new(format!("AutoProjectile_{}", count)),
        ));
        
        println!("Spawned AutoProjectile_{} - check the inspector!", count);
    }
}