//! This example showcases the Entity Inspector plugin from bevy_dev_tools.
//!
//! The Entity Inspector provides a live view of all entities and their components
//! in your Bevy application. It's useful for debugging and understanding the
//! structure of your game world at runtime.
//!
//! Controls:
//! - Press F12 to toggle the Entity Inspector window
//! - Click on entities in the left pane to inspect their components
//! - Use WASD to move the cube around
//! - Press SPACE to jump
//! - Press R to reset the cube position

use bevy::prelude::*;
use bevy::dev_tools::entity_inspector::{EntityInspectorPlugin, InspectorConfig};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Configure the Entity Inspector before adding the plugin
        .insert_resource(InspectorConfig {
            toggle_key: KeyCode::F12,        // Default key
            use_overlay_mode: true,          // Use overlay mode (default)
            //toggle_key: KeyCode::Tab,      // Alternative: use Tab key
            //use_overlay_mode: false,       // Alternative: use separate window
        })
        // Add the Entity Inspector plugin
        .add_plugins(EntityInspectorPlugin)
        // Register custom components for reflection (required for inspector to show component data)
        .register_type::<MovementSpeed>()
        .register_type::<PlayerState>()
        .register_type::<Velocity>()
        .register_type::<Player>()
        .register_type::<Collectible>()
        .add_systems(Startup, setup_scene)
        .add_systems(Update, (move_cube, apply_gravity, handle_input))
        .run();
}

/// Custom component to track player movement speed
#[derive(Component, Reflect)]
#[reflect(Component)]
struct MovementSpeed {
    speed: f32,
    jump_force: f32,
}

/// Custom component to track player state
#[derive(Component, Reflect)]
#[reflect(Component)]
struct PlayerState {
    is_grounded: bool,
    health: i32,
    score: u32,
}

/// Custom component for physics simulation
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Velocity {
    velocity: Vec3,
}

/// Custom component to mark the player entity
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Player;

/// Custom component for game objects that can be collected
#[derive(Component, Reflect)]
#[reflect(Component)]
struct Collectible {
    value: u32,
    collected: bool,
}

/// Setup the 3D scene with a player cube, collectibles, and environment
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn a camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        Name::new("Main Camera"),
    ));

    // Add a light
    commands.spawn((
        DirectionalLight {
            illuminance: 3000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -std::f32::consts::PI / 4.)),
        Name::new("Sun"),
    ));

    // Create the player cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.2, 0.2))),
        Transform::from_xyz(0.0, 2.0, 0.0),
        Player,
        MovementSpeed {
            speed: 5.0,
            jump_force: 8.0,
        },
        PlayerState {
            is_grounded: false,
            health: 100,
            score: 0,
        },
        Velocity {
            velocity: Vec3::ZERO,
        },
        Name::new("Player Cube"),
    ));

    // Create a ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, 0.0, 0.0),
        Name::new("Ground"),
    ));

    // Create some collectible items
    for i in 0..5 {
        let x = (i as f32 - 2.0) * 3.0;
        let z = if i % 2 == 0 { 2.0 } else { -2.0 };
        
        commands.spawn((
            Mesh3d(meshes.add(Sphere::new(0.3))),
            MeshMaterial3d(materials.add(Color::srgb(1.0, 1.0, 0.2))),
            Transform::from_xyz(x, 1.0, z),
            Collectible {
                value: 10 * (i + 1) as u32,
                collected: false,
            },
            Name::new(format!("Collectible {}", i + 1)),
        ));
    }

    // Create some decorative objects
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 3.0, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.6, 0.3, 0.1))),
        Transform::from_xyz(8.0, 1.5, 0.0),
        Name::new("Tree Trunk"),
    ));

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.6, 0.1))),
        Transform::from_xyz(8.0, 4.0, 0.0),
        Name::new("Tree Leaves"),
    ));

    // Add some UI text for instructions
    commands.spawn((
        Text::new("Entity Inspector Example\n\nControls:\n- F12: Toggle Inspector\n- WASD: Move cube\n- SPACE: Jump\n- R: Reset position"),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        TextFont {
            font_size: 16.0,
            ..default()
        },
        TextColor(Color::WHITE),
        Name::new("Instructions UI"),
    ));
}

/// Handle player input for movement and actions
fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut player_query: Query<
        (&mut Transform, &mut Velocity, &mut PlayerState, &MovementSpeed),
        With<Player>,
    >,
    time: Res<Time>,
) {
    if let Ok((mut transform, mut velocity, mut player_state, movement)) = player_query.single_mut() {
        let mut movement_dir = Vec3::ZERO;

        // Handle movement input
        if keyboard_input.pressed(KeyCode::KeyW) {
            movement_dir.z -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            movement_dir.z += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            movement_dir.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            movement_dir.x += 1.0;
        }

        // Apply horizontal movement
        if movement_dir.length() > 0.0 {
            movement_dir = movement_dir.normalize();
            velocity.velocity.x = movement_dir.x * movement.speed;
            velocity.velocity.z = movement_dir.z * movement.speed;
        } else {
            // Apply friction when not moving
            velocity.velocity.x *= 0.8;
            velocity.velocity.z *= 0.8;
        }

        // Handle jumping
        if keyboard_input.just_pressed(KeyCode::Space) && player_state.is_grounded {
            velocity.velocity.y = movement.jump_force;
            player_state.is_grounded = false;
        }

        // Handle reset
        if keyboard_input.just_pressed(KeyCode::KeyR) {
            transform.translation = Vec3::new(0.0, 2.0, 0.0);
            velocity.velocity = Vec3::ZERO;
            player_state.is_grounded = false;
        }
    }
}

/// Move the cube based on its velocity
fn move_cube(
    mut player_query: Query<(&mut Transform, &Velocity), With<Player>>,
    time: Res<Time>,
) {
    if let Ok((mut transform, velocity)) = player_query.single_mut() {
        transform.translation += velocity.velocity * time.delta_secs();
    }
}

/// Apply gravity and ground collision detection
fn apply_gravity(
    mut player_query: Query<(&mut Transform, &mut Velocity, &mut PlayerState), With<Player>>,
    mut collectible_query: Query<(&Transform, &mut Collectible), (With<Collectible>, Without<Player>)>,
    time: Res<Time>,
) {
    if let Ok((mut transform, mut velocity, mut player_state)) = player_query.single_mut() {
        // Apply gravity
        velocity.velocity.y -= 9.81 * time.delta_secs();
        
        // Ground collision (simple)
        if transform.translation.y <= 0.5 {
            transform.translation.y = 0.5;
            velocity.velocity.y = 0.0;
            player_state.is_grounded = true;
        }

        // Keep player in bounds
        transform.translation.x = transform.translation.x.clamp(-10.0, 10.0);
        transform.translation.z = transform.translation.z.clamp(-10.0, 10.0);

        // Check for collectible collision
        for (collectible_transform, mut collectible) in collectible_query.iter_mut() {
            if !collectible.collected {
                let distance = transform.translation.distance(collectible_transform.translation);
                if distance < 1.0 {
                    collectible.collected = true;
                    player_state.score += collectible.value;
                    info!("Collected item worth {} points! Total score: {}", collectible.value, player_state.score);
                }
            }
        }
    }
}