//! Test animation transitions by spawning several meshes and randomly starting
//! transitions.

use bevy::{prelude::*, scene::SceneInstanceReady};
use rand::{seq::SliceRandom, Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Duration;

const GLTF_PATH: &str = "models/animated/Fox.glb";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 3500.0,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, play_transitions)
        .run();
}

#[derive(Component)]
struct Animations {
    node_indices: Vec<AnimationNodeIndex>,
    graph_handle: Handle<AnimationGraph>,
    time_until_transition: f32,
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let scene = SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(GLTF_PATH)));

    let (graph, node_indices) = AnimationGraph::from_clips([
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(GLTF_PATH)),
        asset_server.load(GltfAssetLabel::Animation(1).from_asset(GLTF_PATH)),
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(GLTF_PATH)),
    ]);

    let graph_handle = graphs.add(graph);

    for x in [-160.0, -80.0, 0.0, 80.0, 160.0] {
        commands
            .spawn((
                scene.clone(),
                Transform::from_xyz(x, 0.0, 0.0),
                Animations {
                    node_indices: node_indices.clone(),
                    graph_handle: graph_handle.clone(),
                    time_until_transition: 0.0,
                },
            ))
            .observe(setup_animations);
    }

    commands.spawn((
        Camera3d::default(),
        Projection::Perspective(PerspectiveProjection {
            fov: 10.0_f32.to_radians(),
            ..Default::default()
        }),
        Transform::from_xyz(900.0, 900.0, 900.0).looking_at(Vec3::new(0.0, 30.0, 0.0), Vec3::Y),
    ));
}

fn setup_animations(
    trigger: Trigger<SceneInstanceReady>,
    mut commands: Commands,
    query_animations: Query<&Animations>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer>,
) {
    if let Ok(animations) = query_animations.get(trigger.target()) {
        for child in children.iter_descendants(trigger.target()) {
            if let Ok(mut player) = players.get_mut(child) {
                let mut transitions = AnimationTransitions::new();

                transitions
                    .play(&mut player, animations.node_indices[0], Duration::ZERO)
                    .repeat();

                commands
                    .entity(child)
                    .insert(transitions)
                    .insert(AnimationGraphHandle(animations.graph_handle.clone()));
            }
        }
    }
}

fn play_transitions(
    query_animations: Query<(Entity, &mut Animations)>,
    children: Query<&Children>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    mut rng: Local<Option<ChaCha8Rng>>,
    time: Res<Time>,
) {
    let rng = rng.get_or_insert(ChaCha8Rng::seed_from_u64(729593));

    for (entity, mut animations) in query_animations {
        animations.time_until_transition =
            (animations.time_until_transition - time.delta_secs()).max(0.0);

        if animations.time_until_transition > 0.0 {
            continue;
        }

        // Choose a random animation.
        let Some(&node_index) = animations.node_indices.choose(rng) else {
            continue;
        };

        // Randomize the blend duration.
        let duration: f32 = rng.gen_range(0.2..1.0);

        // Play a new transition after this one has finished blending.
        animations.time_until_transition = duration + 0.5;

        for child in children.iter_descendants(entity) {
            if let Ok((mut player, mut transitions)) = players.get_mut(child) {
                // Don't play the animation if it's already playing. This would
                // look like a bug as it snaps the animation to the start.
                if transitions.get_main_animation() == Some(node_index) {
                    continue;
                }

                transitions
                    .play(&mut player, node_index, Duration::from_secs_f32(duration))
                    .repeat();
            }
        }
    }
}
