//! Plays animations from a skinned glTF.

use std::{f32::consts::PI, time::Duration};

use bevy::{animation::RepeatAnimation, light::CascadeShadowConfigBuilder, prelude::*};

const FOX_PATH: &str = "models/animated/Fox.glb";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, setup_scene_once_loaded)
        .add_systems(Update, keyboard_control)
        .run();
}

#[derive(Resource)]
struct Animations {
    animations: Vec<AnimationNodeIndex>,
    graph_handle: Handle<AnimationGraph>,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Build the animation graph
    let (graph, node_indices) = AnimationGraph::from_clips([
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(FOX_PATH)),
        asset_server.load(GltfAssetLabel::Animation(1).from_asset(FOX_PATH)),
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(FOX_PATH)),
    ]);

    // Keep our animation graph in a Resource so that it can be inserted onto
    // the correct entity once the scene actually loads.
    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        animations: node_indices,
        graph_handle,
    });

    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(100.0, 100.0, 150.0).looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(500000.0, 500000.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Light
    commands.spawn((
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        DirectionalLight {
            shadows_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 200.0,
            maximum_distance: 400.0,
            ..default()
        }
        .build(),
    ));

    // Fox
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(FOX_PATH)),
    ));

    // Instructions

    commands.spawn((
        Text::new(concat!(
            "space: play / pause\n",
            "up / down: playback speed\n",
            "left / right: seek\n",
            "1-3: play N times\n",
            "L: loop forever\n",
            "return: change animation\n",
        )),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the animation.
fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        let mut transitions = AnimationTransitions::new();

        // Make sure to start the animation via the `AnimationTransitions`
        // component. The `AnimationTransitions` component wants to manage all
        // the animations and will get confused if the animations are started
        // directly via the `AnimationPlayer`.
        transitions
            .play(&mut player, animations.animations[0], Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph_handle.clone()))
            .insert(transitions);
    }
}

fn keyboard_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
    mut current_animation: Local<usize>,
) {
    for (mut player, mut transitions) in &mut animation_players {
        let Some((&playing_animation_index, _)) = player.playing_animations().next() else {
            continue;
        };

        if keyboard_input.just_pressed(KeyCode::Space) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            if playing_animation.is_paused() {
                playing_animation.resume();
            } else {
                playing_animation.pause();
            }
        }

        if keyboard_input.just_pressed(KeyCode::ArrowUp) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let speed = playing_animation.speed();
            playing_animation.set_speed(speed * 1.2);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowDown) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let speed = playing_animation.speed();
            playing_animation.set_speed(speed * 0.8);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowLeft) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let elapsed = playing_animation.seek_time();
            playing_animation.seek_to(elapsed - 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::ArrowRight) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            let elapsed = playing_animation.seek_time();
            playing_animation.seek_to(elapsed + 0.1);
        }

        if keyboard_input.just_pressed(KeyCode::Enter) {
            *current_animation = (*current_animation + 1) % animations.animations.len();

            transitions
                .play(
                    &mut player,
                    animations.animations[*current_animation],
                    Duration::from_millis(250),
                )
                .repeat();
        }

        if keyboard_input.just_pressed(KeyCode::Digit1) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(1))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::Digit2) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(2))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::Digit3) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(3))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::KeyL) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation.set_repeat(RepeatAnimation::Forever);
        }
    }
}
