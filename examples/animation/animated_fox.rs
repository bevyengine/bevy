//! Plays animations from a skinned glTF.

use std::f32::consts::PI;
use std::time::Duration;

use bevy::{pbr::CascadeShadowConfigBuilder, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 150.0,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (setup_scene_once_loaded, keyboard_animation_control),
        )
        .run();
}

#[derive(Resource)]
struct Animations(Vec<Handle<AnimationClip>>);

#[derive(Component)]
struct AnimationNodes(Vec<AnimationNodeIndex>);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Insert a resource with the current scene information
    commands.insert_resource(Animations(vec![
        asset_server.load("models/animated/Fox.glb#Animation2"),
        asset_server.load("models/animated/Fox.glb#Animation1"),
        asset_server.load("models/animated/Fox.glb#Animation0"),
    ]));

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(100.0, 100.0, 150.0)
            .looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
        ..default()
    });

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(shape::Plane::from_size(500000.0)),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
            illuminance: 2000.0,
            shadows_enabled: true,
            ..default()
        },
        cascade_shadow_config: CascadeShadowConfigBuilder {
            first_cascade_far_bound: 200.0,
            maximum_distance: 400.0,
            ..default()
        }
        .into(),
        ..default()
    });

    // Fox
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/animated/Fox.glb#Scene0"),
        ..default()
    });

    println!("Animation controls:");
    println!("  - spacebar: play / pause");
    println!("  - arrow up / down: speed up / slow down animation playback");
    println!("  - arrow left / right: seek backward / forward");
    println!("  - digit 1 / 3 / 5: play the animation <digit> times");
    println!("  - L: loop the animation forever");
    println!("  - return: change animation");
}

// Once the scene is loaded, start the animation
fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    mut players: Query<(Entity, &mut AnimationGraph), Added<AnimationGraph>>,
) {
    for (entity, mut player) in &mut players {
        let root_node = player.root_node();
        let mut animation_nodes = AnimationNodes(vec![]);
        for (clip_index, clip) in animations.0.iter().enumerate() {
            let node = player.add_clip_node_from(root_node, clip.clone_weak());
            player[node]
                .set_weight(if clip_index == 0 { 1.0 } else { 0.0 })
                .repeat_forever()
                .play();
            animation_nodes.0.push(node);
        }

        commands
            .entity(entity)
            .insert(AnimationTransitions::new())
            .insert(animation_nodes);
    }
}

fn keyboard_animation_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<(
        &mut AnimationGraph,
        &mut AnimationTransitions,
        &AnimationNodes,
    )>,
    animations: Res<Animations>,
    mut current_animation: Local<usize>,
    time: Res<Time>,
) {
    for (mut player, mut transitions, nodes) in &mut animation_players {
        let node_index = nodes.0[*current_animation];
        if keyboard_input.just_pressed(KeyCode::Enter) {
            let next_animation = (*current_animation + 1) % animations.0.len();
            let duration = Duration::from_millis(250);
            transitions.transition_from_current(&time, &player, node_index, 0.0, duration);
            transitions.transition_from_current(
                &time,
                &player,
                nodes.0[next_animation],
                1.0,
                duration,
            );

            *current_animation = next_animation;
        }

        let node = &mut player[node_index];

        if keyboard_input.just_pressed(KeyCode::Space) {
            if node.paused() {
                node.play();
            } else {
                node.pause();
            }
        }

        if keyboard_input.just_pressed(KeyCode::ArrowUp) {
            let speed = node.speed();
            node.set_speed(speed * 1.2);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowDown) {
            let speed = node.speed();
            node.set_speed(speed * 0.8);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
            let elapsed = node.seek_time();
            node.set_seek_time(elapsed - 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowRight) {
            let elapsed = node.seek_time();
            node.set_seek_time(elapsed + 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Digit1) {
            node.repeat_n(1);
            node.restart();
        }

        if keyboard_input.just_pressed(KeyCode::Digit3) {
            node.repeat_n(3);
            node.restart();
        }

        if keyboard_input.just_pressed(KeyCode::Digit5) {
            node.repeat_n(5);
            node.restart();
        }

        if keyboard_input.just_pressed(KeyCode::KeyL) {
            node.repeat_forever();
        }
    }
}
