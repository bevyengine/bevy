//! Demonstrates loading and playing multiple animations from a GLTF file.
//!
//! This example shows how to:
//! - Load a GLTF file with multiple animations
//! - Switch between different animation clips
//! - Control animation playback (play, pause, loop)
//! - Display animation information in the UI
//! cargo run --example gltf_viewer_main

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                setup_scene_once_loaded,
                keyboard_animation_control,
                camera_controller,
                update_ui,
            ),
        )
        .run();
}

#[derive(Component)]
struct AnimationsLoaded {
    animations: Vec<AnimationNodeIndex>,
}

#[derive(Resource)]
struct Animations {
    clips: Vec<Handle<AnimationClip>>,
    names: Vec<String>,
    current: usize,
}

#[derive(Component)]
struct CameraController {
    pub orbit_distance: f32,
    pub orbit_focus: Vec3,
    pub orbit_rotation: Vec2,
}

impl Default for CameraController {
    fn default() -> Self {
        Self {
            orbit_distance: 200.0,
            orbit_focus: Vec3::new(0.0, 1.0, 0.0),
            orbit_rotation: Vec2::ZERO,
        }
    }
}

#[derive(Component)]
struct AnimationEntityLink;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera positioned to see the model with orbit controls
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 2.0, 200.0).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
        CameraController::default(),
    ));

    // Ambient light for better visibility
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 300.0,
        affects_lightmapped_meshes: true,
    });

    // Directional light
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 10000.0,
            ..default()
        },
        Transform {
            translation: Vec3::new(0.0, 5.0, 2.0),
            rotation: Quat::from_rotation_x(-std::f32::consts::FRAC_PI_4),
            ..default()
        },
    ));

    // Load the GLTF file
    let gltf_handle: Handle<Scene> = asset_server.load("models/GLTFfold/main.gltf#Scene0");

    commands.spawn((
        SceneRoot(gltf_handle),
        Transform::from_scale(Vec3::splat(1.0)), // Ensure proper scale
        Visibility::default(),
        AnimationEntityLink,
    ));

    // Load animation clips using the actual names from your GLTF file
    let animations = vec![
        asset_server.load("models/GLTFfold/main.gltf#Animation0"),
        asset_server.load("models/GLTFfold/main.gltf#Animation1"),
        asset_server.load("models/GLTFfold/main.gltf#Animation2"),
    ];

    commands.insert_resource(Animations {
        clips: animations,
        names: vec![
            "Animation0 (Running)".to_string(),
            "Animation1 (Walking)".to_string(),
            "Animation2 (Dying)".to_string(),
        ],
        current: 0,
    });

    // UI
    commands.spawn((
        Text::new("Loading..."),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        TextColor(Color::WHITE),
        TextFont {
            font_size: 20.0,
            ..default()
        },
    ));
}

fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    for (entity, mut player) in &mut players {
        let mut graph = AnimationGraph::new();
        let mut animation_indices = Vec::new();

        // Add all animations to the graph
        for animation_clip in &animations.clips {
            let animation_index = graph.add_clip(animation_clip.clone(), 1.0, graph.root);
            animation_indices.push(animation_index);
        }

        let graph_handle = animation_graphs.add(graph);

        // Add our component to track the animations
        commands.entity(entity).insert(AnimationsLoaded {
            animations: animation_indices.clone(),
        });

        // Add the animation graph handle using the proper component
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(graph_handle));

        // Start the first animation automatically
        if let Some(&first_animation) = animation_indices.first() {
            player.start(first_animation).repeat();
            println!("Started animation: {:?}", first_animation);
        }

        *done = true;
        break;
    }
}

fn keyboard_animation_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animations_res: ResMut<Animations>,
    mut query: Query<(&mut AnimationPlayer, &AnimationsLoaded)>,
) {
    if let Ok((mut player, animations_loaded)) = query.single_mut() {
        let mut changed = false;

        // Switch animations with number keys
        if keyboard_input.just_pressed(KeyCode::Digit1) && animations_loaded.animations.len() > 0 {
            animations_res.current = 0;
            changed = true;
        }
        if keyboard_input.just_pressed(KeyCode::Digit2) && animations_loaded.animations.len() > 1 {
            animations_res.current = 1;
            changed = true;
        }
        if keyboard_input.just_pressed(KeyCode::Digit3) && animations_loaded.animations.len() > 2 {
            animations_res.current = 2;
            changed = true;
        }

        // Play/Pause controls
        if keyboard_input.just_pressed(KeyCode::Space) {
            if player.all_paused() {
                player.resume_all();
            } else {
                player.pause_all();
            }
        }

        // Reset animation
        if keyboard_input.just_pressed(KeyCode::KeyR) {
            if let Some(&current_animation) =
                animations_loaded.animations.get(animations_res.current)
            {
                player.start(current_animation).repeat();
            }
        }

        // Speed controls
        if keyboard_input.just_pressed(KeyCode::Equal) {
            // Speed up all animations
            for &animation in &animations_loaded.animations {
                if let Some(animation_ref) = player.animation_mut(animation) {
                    let current_speed = animation_ref.speed();
                    animation_ref.set_speed(current_speed * 1.2);
                }
            }
        }
        if keyboard_input.just_pressed(KeyCode::Minus) {
            // Slow down all animations
            for &animation in &animations_loaded.animations {
                if let Some(animation_ref) = player.animation_mut(animation) {
                    let current_speed = animation_ref.speed();
                    animation_ref.set_speed(current_speed * 0.8);
                }
            }
        }

        // Change animation if requested
        if changed {
            if let Some(&new_animation) = animations_loaded.animations.get(animations_res.current) {
                player.start(new_animation).repeat();
                println!(
                    "Switched to animation {}: {:?}",
                    animations_res.current, new_animation
                );
            }
        }
    }
}

fn camera_controller(
    time: Res<Time>,
    mut mouse_events: EventReader<MouseMotion>,
    mut scroll_events: EventReader<MouseWheel>,
    input_mouse: Res<ButtonInput<MouseButton>>,
    keyboards: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
    let Ok((mut transform, mut controller)) = query.single_mut() else {
        return;
    };

    let mut rotation_move = Vec2::ZERO;
    let mut scroll = 0.0;
    let dt = time.delta_secs();

    // Handle mouse rotation (hold right click and drag)
    if input_mouse.pressed(MouseButton::Right) {
        for mouse_event in mouse_events.read() {
            rotation_move += mouse_event.delta;
        }
    } else {
        // Clear mouse events if not using them
        mouse_events.clear();
    }

    // Handle scroll wheel for zoom
    for scroll_event in scroll_events.read() {
        scroll += scroll_event.y;
    }

    // Handle keyboard controls
    let mut keyboard_move = Vec2::ZERO;
    let mut zoom_change = 0.0;

    if keyboards.pressed(KeyCode::ArrowLeft) || keyboards.pressed(KeyCode::KeyA) {
        keyboard_move.x -= 100.0 * dt;
    }
    if keyboards.pressed(KeyCode::ArrowRight) || keyboards.pressed(KeyCode::KeyD) {
        keyboard_move.x += 100.0 * dt;
    }
    if keyboards.pressed(KeyCode::ArrowUp) || keyboards.pressed(KeyCode::KeyW) {
        keyboard_move.y += 100.0 * dt;
    }
    if keyboards.pressed(KeyCode::ArrowDown) || keyboards.pressed(KeyCode::KeyS) {
        keyboard_move.y -= 100.0 * dt;
    }
    if keyboards.pressed(KeyCode::KeyQ) {
        zoom_change -= 5.0 * dt;
    }
    if keyboards.pressed(KeyCode::KeyE) {
        zoom_change += 5.0 * dt;
    }

    // Apply rotation from mouse and keyboard
    let total_rotation = (rotation_move * 0.003) + keyboard_move;
    controller.orbit_rotation.x += total_rotation.x;
    controller.orbit_rotation.y += total_rotation.y;

    // Clamp vertical rotation to avoid flipping
    controller.orbit_rotation.y = controller.orbit_rotation.y.clamp(-1.54, 1.54);

    // Apply zoom from scroll wheel and keyboard
    let total_zoom = scroll * 0.5 + zoom_change;
    controller.orbit_distance -= total_zoom;
    controller.orbit_distance = controller.orbit_distance.clamp(1.0, 200.0);

    // Calculate new camera position
    let rotation_quat = Quat::from_axis_angle(Vec3::Y, controller.orbit_rotation.x)
        * Quat::from_axis_angle(Vec3::X, controller.orbit_rotation.y);

    let camera_pos =
        controller.orbit_focus + rotation_quat * Vec3::new(0.0, 0.0, controller.orbit_distance);

    transform.translation = camera_pos;
    transform.look_at(controller.orbit_focus, Vec3::Y);
}

fn update_ui(
    animations: Res<Animations>,
    mut text_query: Query<&mut Text>,
    player_query: Query<(&AnimationPlayer, &AnimationsLoaded)>,
) {
    if let Ok(mut text) = text_query.single_mut() {
        if let Ok((player, animations_loaded)) = player_query.single() {
            let current_animation_index = animations_loaded
                .animations
                .get(animations.current)
                .copied();

            let (current_time, speed, is_paused) =
                if let Some(animation_index) = current_animation_index {
                    if let Some(animation) = player.animation(animation_index) {
                        (
                            animation.seek_time(),
                            animation.speed(),
                            animation.is_paused(),
                        )
                    } else {
                        (0.0, 1.0, true)
                    }
                } else {
                    (0.0, 1.0, true)
                };

            // In Bevy 0.16, Text is now a simple string wrapper
            *text = Text::new(format!(
                "GLTF Multi-Animation Example\n\
                \n\
                Current Animation: {} ({})\n\
                Time: {:.2}s\n\
                Speed: {:.2}x\n\
                Status: {}\n\
                \n\
                Animation Controls:\n\
                1, 2, 3 - Switch animations\n\
                SPACE - Play/Pause\n\
                R - Reset to beginning\n\
                +/- - Speed up/slow down\n\
                \n\
                Camera Controls:\n\
                Right Click + Drag - Rotate camera\n\
                Mouse Wheel - Zoom in/out\n\
                WASD/Arrow Keys - Rotate camera\n\
                Q/E - Zoom in/out\n\
                \n\
                Available Animations:\n\
                {}",
                animations.current + 1,
                animations
                    .names
                    .get(animations.current)
                    .unwrap_or(&"Unknown".to_string()),
                current_time,
                speed,
                if is_paused { "Paused" } else { "Playing" },
                animations
                    .names
                    .iter()
                    .enumerate()
                    .map(|(i, name)| format!("{}. {}", i + 1, name))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        } else {
            *text = Text::new("Loading animations...");
        }
    }
}
