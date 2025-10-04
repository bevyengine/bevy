//! Plays animations from a skinned glTF.

use std::{f32::consts::PI, time::Duration};

use bevy::{
    animation::{AnimationEvent, AnimationTargetId},
    color::palettes::css::WHITE,
    light::CascadeShadowConfigBuilder,
    prelude::*,
};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

const FOX_PATH: &str = "models/animated/Fox.glb";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .init_resource::<ParticleAssets>()
        .init_resource::<FoxFeetTargets>()
        .add_systems(Startup, setup)
        .add_systems(Update, setup_scene_once_loaded)
        .add_systems(Update, simulate_particles)
        .add_observer(observe_on_step)
        .run();
}

#[derive(Resource)]
struct SeededRng(ChaCha8Rng);

#[derive(Resource)]
struct Animations {
    index: AnimationNodeIndex,
    graph_handle: Handle<AnimationGraph>,
}

#[derive(AnimationEvent, Reflect, Clone)]
struct Step;

fn observe_on_step(
    step: On<Step>,
    particle: Res<ParticleAssets>,
    mut commands: Commands,
    transforms: Query<&GlobalTransform>,
    mut seeded_rng: ResMut<SeededRng>,
) -> Result {
    let translation = transforms
        .get(step.trigger().animation_player)?
        .translation();
    // Spawn a bunch of particles.
    for _ in 0..14 {
        let horizontal = seeded_rng.0.random::<Dir2>() * seeded_rng.0.random_range(8.0..12.0);
        let vertical = seeded_rng.0.random_range(0.0..4.0);
        let size = seeded_rng.0.random_range(0.2..1.0);

        commands.spawn((
            Particle {
                lifetime_timer: Timer::from_seconds(
                    seeded_rng.0.random_range(0.2..0.6),
                    TimerMode::Once,
                ),
                size,
                velocity: Vec3::new(horizontal.x, vertical, horizontal.y) * 10.0,
            },
            Mesh3d(particle.mesh.clone()),
            MeshMaterial3d(particle.material.clone()),
            Transform {
                translation,
                scale: Vec3::splat(size),
                ..Default::default()
            },
        ));
    }
    Ok(())
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Build the animation graph
    let (graph, index) = AnimationGraph::from_clip(
        // We specifically want the "run" animation, which is the third one.
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(FOX_PATH)),
    );

    // Insert a resource with the current scene information
    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        index,
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

    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);
    commands.insert_resource(SeededRng(seeded_rng));
}

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the animation.
fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    feet: Res<FoxFeetTargets>,
    graphs: Res<Assets<AnimationGraph>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    fn get_clip<'a>(
        node: AnimationNodeIndex,
        graph: &AnimationGraph,
        clips: &'a mut Assets<AnimationClip>,
    ) -> &'a mut AnimationClip {
        let node = graph.get(node).unwrap();
        let clip = match &node.node_type {
            AnimationNodeType::Clip(handle) => clips.get_mut(handle),
            _ => unreachable!(),
        };
        clip.unwrap()
    }

    for (entity, mut player) in &mut players {
        // Send `OnStep` events once the fox feet hits the ground in the running animation.

        let graph = graphs.get(&animations.graph_handle).unwrap();
        let running_animation = get_clip(animations.index, graph, &mut clips);

        // You can determine the time an event should trigger if you know witch frame it occurs and
        // the frame rate of the animation. Let's say we want to trigger an event at frame 15,
        // and the animation has a frame rate of 24 fps, then time = 15 / 24 = 0.625.
        running_animation.add_event_to_target(feet.front_left, 0.625, Step);
        running_animation.add_event_to_target(feet.front_right, 0.5, Step);
        running_animation.add_event_to_target(feet.back_left, 0.0, Step);
        running_animation.add_event_to_target(feet.back_right, 0.125, Step);

        // Start the animation

        let mut transitions = AnimationTransitions::new();

        // Make sure to start the animation via the `AnimationTransitions`
        // component. The `AnimationTransitions` component wants to manage all
        // the animations and will get confused if the animations are started
        // directly via the `AnimationPlayer`.
        transitions
            .play(&mut player, animations.index, Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animations.graph_handle.clone()))
            .insert(transitions);
    }
}

fn simulate_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Particle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut particle) in &mut query {
        if particle.lifetime_timer.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
            return;
        }

        transform.translation += particle.velocity * time.delta_secs();
        transform.scale = Vec3::splat(particle.size.lerp(0.0, particle.lifetime_timer.fraction()));
        particle
            .velocity
            .smooth_nudge(&Vec3::ZERO, 4.0, time.delta_secs());
    }
}

#[derive(Component)]
struct Particle {
    lifetime_timer: Timer,
    size: f32,
    velocity: Vec3,
}

#[derive(Resource)]
struct ParticleAssets {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for ParticleAssets {
    fn from_world(world: &mut World) -> Self {
        Self {
            mesh: world.add_asset::<Mesh>(Sphere::new(10.0)),
            material: world.add_asset::<StandardMaterial>(StandardMaterial {
                base_color: WHITE.into(),
                ..Default::default()
            }),
        }
    }
}

/// Stores the `AnimationTargetId`s of the fox's feet
#[derive(Resource)]
struct FoxFeetTargets {
    front_right: AnimationTargetId,
    front_left: AnimationTargetId,
    back_left: AnimationTargetId,
    back_right: AnimationTargetId,
}

impl Default for FoxFeetTargets {
    fn default() -> Self {
        let hip_node = ["root", "_rootJoint", "b_Root_00", "b_Hip_01"];
        let front_left_foot = hip_node.iter().chain(
            [
                "b_Spine01_02",
                "b_Spine02_03",
                "b_LeftUpperArm_09",
                "b_LeftForeArm_010",
                "b_LeftHand_011",
            ]
            .iter(),
        );
        let front_right_foot = hip_node.iter().chain(
            [
                "b_Spine01_02",
                "b_Spine02_03",
                "b_RightUpperArm_06",
                "b_RightForeArm_07",
                "b_RightHand_08",
            ]
            .iter(),
        );
        let back_left_foot = hip_node.iter().chain(
            [
                "b_LeftLeg01_015",
                "b_LeftLeg02_016",
                "b_LeftFoot01_017",
                "b_LeftFoot02_018",
            ]
            .iter(),
        );
        let back_right_foot = hip_node.iter().chain(
            [
                "b_RightLeg01_019",
                "b_RightLeg02_020",
                "b_RightFoot01_021",
                "b_RightFoot02_022",
            ]
            .iter(),
        );
        Self {
            front_left: AnimationTargetId::from_iter(front_left_foot),
            front_right: AnimationTargetId::from_iter(front_right_foot),
            back_left: AnimationTargetId::from_iter(back_left_foot),
            back_right: AnimationTargetId::from_iter(back_right_foot),
        }
    }
}
