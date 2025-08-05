//! Example target application for the remote inspector
//!
//! This is a simple Bevy application that enables bevy_remote and creates
//! some entities that can be inspected by the remote inspector.
//!
//! Run this first, then run the inspector to connect to it.

use bevy::prelude::*;
use std::time::Duration;
use bevy::remote::{RemotePlugin, http::RemoteHttpPlugin};

/// Custom component for demonstration
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Player {
    pub health: i32,
    pub speed: f32,
    pub level: u32,
}

/// Custom component for enemies
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Enemy {
    pub damage: i32,
    pub health: i32,
    pub ai_type: String,
}

/// Custom component for items
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Item {
    pub name: String,
    pub value: i32,
    pub stackable: bool,
}

/// Movement component for animated entities
#[derive(Component)]
struct Mover {
    pub speed: f32,
    pub direction: Vec3,
}

fn main() {
    println!("Starting target application for remote inspector");
    println!("bevy_remote will be available at http://localhost:15702");
    println!("Start the remote inspector to connect and view entities");
    
    App::new()
        .add_plugins(DefaultPlugins)
        
        // Enable bevy_remote for inspector connection
        .add_plugins(bevy::remote::RemotePlugin::default())
        .add_plugins(RemoteHttpPlugin::default())
        
        // Register custom components for reflection
        .register_type::<Player>()
        .register_type::<Enemy>()
        .register_type::<Item>()
        
        // Register built-in components for reflection
        .register_type::<Transform>()
        .register_type::<Name>()
        
        // Setup systems
        .add_systems(Startup, setup_demo_scene)
        .add_systems(Update, (
            move_entities,
            update_player_stats,
            spawn_periodic_entities,
        ))
        
        .run();
}

/// Set up a demo scene with various entities
fn setup_demo_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    println!("Setting up demo scene with entities...");
    
    // Spawn a 3D camera to view the entities
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        Name::new("MainCamera"),
    ));
    
    // Create some materials for visual distinction
    let player_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.8, 0.2), // Green
        ..default()
    });
    let enemy_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.2, 0.2), // Red
        ..default()
    });
    let item_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.2, 0.8), // Blue
        ..default()
    });
    let basic_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.6, 0.6, 0.6), // Gray
        ..default()
    });
    
    // Create meshes
    let cube_mesh = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let sphere_mesh = meshes.add(Sphere::new(0.5));
    
    // Add some lighting
    commands.spawn((
        DirectionalLightBundle {
            directional_light: DirectionalLight {
                color: Color::WHITE,
                illuminance: 3000.0,
                ..default()
            },
            transform: Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
            ..default()
        },
    ));
    
    // Spawn player entity with visual representation
    commands.spawn((
        Player {
            health: 100,
            speed: 5.0,
            level: 1,
        },
        PbrBundle {
            mesh: cube_mesh.clone(),
            material: player_material,
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Mover {
            speed: 2.0,
            direction: Vec3::new(1.0, 0.0, 0.0),
        },
        Name::new("Player"),
    ));
    
    // Spawn some enemies with visual representation
    for i in 0..3 {
        let x = (i as f32 - 1.0) * 5.0;
        commands.spawn((
            Enemy {
                damage: 20 + i * 5,
                health: 50 + i * 10,
                ai_type: match i {
                    0 => "Aggressive".to_string(),
                    1 => "Defensive".to_string(),
                    _ => "Patrol".to_string(),
                },
            },
            PbrBundle {
                mesh: cube_mesh.clone(),
                material: enemy_material.clone(),
                transform: Transform::from_xyz(x, 3.0, 0.0),
                ..default()
            },
            Mover {
                speed: 1.0 + i as f32 * 0.5,
                direction: Vec3::new(0.0, -1.0, 0.0),
            },
            Name::new(format!("Enemy_{}", i + 1)),
        ));
    }
    
    // Spawn some items with visual representation
    let items = [
        ("Health Potion", 50, true),
        ("Magic Sword", 200, false),
        ("Gold Coin", 1, true),
        ("Shield", 150, false),
    ];
    
    for (i, (name, value, stackable)) in items.iter().enumerate() {
        let angle = i as f32 * std::f32::consts::PI * 0.5;
        let x = angle.cos() * 8.0;
        let y = angle.sin() * 8.0;
        
        commands.spawn((
            Item {
                name: name.to_string(),
                value: *value,
                stackable: *stackable,
            },
            PbrBundle {
                mesh: sphere_mesh.clone(),
                material: item_material.clone(),
                transform: Transform::from_xyz(x, y, 0.0),
                ..default()
            },
            Name::new(name.to_string()),
        ));
    }
    
    // Spawn some basic entities with just Transform
    for i in 0..5 {
        let x = (i as f32 - 2.0) * 2.0;
        commands.spawn((
            PbrBundle {
                mesh: cube_mesh.clone(),
                material: basic_material.clone(),
                transform: Transform::from_xyz(x, -5.0, 0.0),
                ..default()
            },
            Name::new(format!("BasicEntity_{}", i + 1)),
        ));
    }
    
    println!("Demo scene setup complete!");
    println!("   - 1 Camera");
    println!("   - 1 Player");
    println!("   - 3 Enemies");
    println!("   - 4 Items");
    println!("   - 5 Basic entities");
    println!("Total: {} entities created", 1 + 1 + 3 + 4 + 5);
    println!();
    println!("What you'll see:");
    println!("   Visually: Green player cube, red enemy cubes, blue item spheres");
    println!("   In inspector: Entities moving around (Transform updates)");
    println!("   In inspector: Player health regenerating and speed changing");
    println!("   In inspector: Player leveling up every 30 seconds");
    println!("   In inspector: New colorful items spawning every 10 seconds");
    println!("   In inspector: Different component types: Player, Enemy, Item, Transform, Name");
    println!();
}

/// Move entities around to demonstrate live updates
fn move_entities(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &Mover)>,
) {
    for (mut transform, mover) in query.iter_mut() {
        // Move entity
        transform.translation += mover.direction * mover.speed * time.delta_secs();
        
        // Bounce off boundaries
        if transform.translation.x.abs() > 10.0 {
            transform.translation.x = transform.translation.x.signum() * 10.0;
        }
        if transform.translation.y.abs() > 10.0 {
            transform.translation.y = transform.translation.y.signum() * 10.0;
        }
    }
}

/// Update player stats over time to show live data changes
fn update_player_stats(
    time: Res<Time>,
    mut player_query: Query<&mut Player>,
) {
    for mut player in player_query.iter_mut() {
        // Simulate health regeneration
        if player.health < 100 {
            player.health = (player.health + 1).min(100);
        }
        
        // Oscillate speed for demonstration
        let time_factor = time.elapsed_secs().sin();
        player.speed = 5.0 + time_factor * 2.0;
        
        // Level up occasionally (every 30 seconds)
        if time.elapsed_secs() as u32 / 30 > player.level - 1 {
            player.level += 1;
            println!("🎉 Player leveled up to level {}!", player.level);
        }
    }
}

/// Periodically spawn new entities to demonstrate dynamic changes
fn spawn_periodic_entities(
    time: Res<Time>,
    mut commands: Commands,
    mut timer: Local<Timer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Initialize timer on first run
    if timer.duration() == Duration::ZERO {
        *timer = Timer::from_seconds(10.0, TimerMode::Repeating);
    }
    
    timer.tick(time.delta());
    
    if timer.just_finished() {
        let entity_count = time.elapsed_secs() as u32 / 10;
        let x = ((entity_count as f32) * 1.5).sin() * 6.0;
        let y = ((entity_count as f32) * 1.2).cos() * 6.0;
        
        // Create a dynamic material with a unique color
        let dynamic_material = materials.add(StandardMaterial {
            base_color: Color::srgb(
                ((entity_count as f32 * 0.3).sin() + 1.0) * 0.5,
                ((entity_count as f32 * 0.5).sin() + 1.0) * 0.5,
                ((entity_count as f32 * 0.7).sin() + 1.0) * 0.5,
            ),
            ..default()
        });
        
        commands.spawn((
            Item {
                name: format!("DynamicItem_{}", entity_count),
                value: (entity_count * 10) as i32,
                stackable: entity_count % 2 == 0,
            },
            PbrBundle {
                mesh: meshes.add(Sphere::new(0.3)),
                material: dynamic_material,
                transform: Transform::from_xyz(x, y, 0.0),
                ..default()
            },
            Name::new(format!("DynamicItem_{}", entity_count)),
        ));
        
        println!("✨ Spawned dynamic entity: DynamicItem_{}", entity_count);
    }
}