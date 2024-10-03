//! Demonstrate how to use animation events with an animated character.

use std::{f32::consts::PI, time::Duration};

use bevy::{
    animation::AnimationTargetId,
    color::palettes::css::{ALICE_BLUE, WHITE},
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
};
use rand::{thread_rng, Rng};

const DETECTIVE_PATH: &str = "models/animated/buntective.glb";

fn main() {
    App::new()
        .register_type::<(OnLanded, OnJumped, OnStep)>()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
        })
        .init_resource::<ParticleAssets>()
        .init_resource::<BoneTargets>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (setup_scene_once_loaded, jump_input, simulate_particles),
        )
        .observe(OnLanded::observer)
        .observe(OnJumped::observer)
        .observe(OnStep::observer)
        .run();
}

#[derive(Resource)]
struct Animations {
    walk: AnimationNodeIndex,
    jump: AnimationNodeIndex,
    graph: Handle<AnimationGraph>,
}

#[derive(Resource)]
struct BoneTargets {
    root: AnimationTargetId,
    right_foot: AnimationTargetId,
    left_foot: AnimationTargetId,
}

impl Default for BoneTargets {
    fn default() -> Self {
        Self {
            root: AnimationTargetId::from_iter(["Armature"]),
            right_foot: AnimationTargetId::from_iter([
                "Armature",
                "pelvis",
                "upperleg.r",
                "lowerleg.r",
                "foot.r",
            ]),
            left_foot: AnimationTargetId::from_iter([
                "Armature",
                "pelvis",
                "upperleg.l",
                "lowerleg.l",
                "foot.l",
            ]),
        }
    }
}

#[derive(Component)]
struct Particle {
    lifeteime: Timer,
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
            mesh: world.resource_mut::<Assets<Mesh>>().add(Sphere::new(0.25)),
            material: world
                .resource_mut::<Assets<StandardMaterial>>()
                .add(StandardMaterial {
                    base_color: WHITE.into(),
                    ..Default::default()
                }),
        }
    }
}

#[derive(Event, Reflect, Clone)]
#[reflect(AnimationEvent)]
struct OnJumped;

impl AnimationEvent for OnJumped {
    fn trigger(&self, _time: f32, target: Entity, world: &mut World) {
        world.entity_mut(target).trigger(Self);
    }

    fn clone_value(&self) -> Box<dyn AnimationEvent> {
        Box::new(Self)
    }
}

impl OnJumped {
    // Spawn particles once the detective jumps.
    fn observer(
        trigger: Trigger<Self>,
        mut commands: Commands,
        particle: Res<ParticleAssets>,
        transforms: Query<&GlobalTransform>,
    ) {
        let translation = transforms.get(trigger.entity()).unwrap().translation();
        let mut rng = thread_rng();
        // Vertical
        for _ in 0..5 {
            let horizontal = rng.gen::<Dir2>() * rng.gen_range(0.0..0.2);
            let vertical = rng.gen_range(0.0..10.0);
            let size = rng.gen_range(0.5..1.0);
            commands.queue(spawn_particle(
                particle.mesh.clone(),
                particle.material.clone(),
                translation.reject_from_normalized(Vec3::Y),
                rng.gen_range(0.1..0.5),
                size,
                Vec3::new(horizontal.x, vertical, horizontal.y),
            ));
        }
        // Horizontal
        for _ in 0..20 {
            let horizontal = rng.gen::<Dir2>() * rng.gen_range(6.0..8.0);
            let vertical = rng.gen_range(0.0..2.0);
            let size = rng.gen_range(0.5..1.0);
            commands.queue(spawn_particle(
                particle.mesh.clone(),
                particle.material.clone(),
                translation.reject_from_normalized(Vec3::Y),
                rng.gen_range(0.5..1.0),
                size,
                Vec3::new(horizontal.x, vertical, horizontal.y),
            ));
        }
    }
}

#[derive(Event, Reflect, Clone)]
#[reflect(AnimationEvent)]
struct OnLanded;

impl AnimationEvent for OnLanded {
    fn trigger(&self, _time: f32, target: Entity, world: &mut World) {
        world.entity_mut(target).trigger(Self);
    }

    fn clone_value(&self) -> Box<dyn AnimationEvent> {
        Box::new(Self)
    }
}

impl OnLanded {
    // Spawn particles once the detective lands.
    fn observer(
        trigger: Trigger<Self>,
        mut commands: Commands,
        particle: Res<ParticleAssets>,
        animations: Res<Animations>,
        mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
        transforms: Query<&GlobalTransform>,
    ) {
        // Transition to the `walk` animation.
        let (mut player, mut transitions) = players.get_mut(trigger.entity()).unwrap();
        transitions
            .play(&mut player, animations.walk, Duration::from_secs_f32(0.8))
            .repeat();

        let translation = transforms.get(trigger.entity()).unwrap().translation();
        let mut rng = thread_rng();
        for _ in 0..25 {
            let horizontal = rng.gen::<Dir2>() * rng.gen_range(8.0..12.0);
            let vertical = rng.gen_range(0.0..4.0);
            let size = rng.gen_range(0.5..1.0);
            commands.queue(spawn_particle(
                particle.mesh.clone(),
                particle.material.clone(),
                translation.reject_from_normalized(Vec3::Y),
                rng.gen_range(0.5..1.0),
                size,
                Vec3::new(horizontal.x, vertical, horizontal.y),
            ));
        }
    }
}

#[derive(Event, Resource, Reflect, Clone)]
#[reflect(AnimationEvent)]
enum OnStep {
    Start,
    End,
}

impl AnimationEvent for OnStep {
    fn trigger(&self, _time: f32, target: Entity, world: &mut World) {
        world.entity_mut(target).trigger(self.clone());
    }

    fn clone_value(&self) -> Box<dyn AnimationEvent> {
        Box::new(self.clone())
    }
}

impl OnStep {
    // Spawn particles on each step the detective takes.
    fn observer(
        trigger: Trigger<Self>,
        mut commands: Commands,
        particle: Res<ParticleAssets>,
        transforms: Query<&GlobalTransform>,
    ) {
        let translation = transforms.get(trigger.entity()).unwrap().translation();
        let mut rng = thread_rng();
        for _ in 0..25 {
            let velocity = match trigger.event() {
                OnStep::Start => {
                    let horizontal = rng.gen::<Dir2>() * rng.gen_range(1.0..2.0);
                    let vertical = rng.gen_range(0.0..2.0);
                    Vec3::new(horizontal.x, vertical, horizontal.y)
                }
                OnStep::End => Vec3::new(
                    rng.gen_range(-0.2..0.2),
                    rng.gen_range(0.0..2.0),
                    -rng.gen_range(0.0..10.0),
                ),
            };
            commands.queue(spawn_particle(
                particle.mesh.clone(),
                particle.material.clone(),
                translation.reject_from_normalized(Vec3::Y),
                rng.gen_range(0.1..0.5),
                rng.gen_range(0.1..1.0),
                velocity,
            ));
        }
    }
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
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(DETECTIVE_PATH)), // walk animation
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(DETECTIVE_PATH)), // jump animation
    ]);

    // Insert a resource with the current scene information
    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        walk: node_indices[0],
        jump: node_indices[1],
        graph: graph_handle,
    });

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(2.0, 3.0, 5.0).looking_at(Vec3::new(0.0, 1.5, 0.0), Vec3::Y),
        camera: Camera {
            clear_color: ClearColorConfig::Custom(ALICE_BLUE.into()),
            ..Default::default()
        },
        ..default()
    });

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
            first_cascade_far_bound: 2.0,
            maximum_distance: 10.0,
            ..default()
        }
        .build(),
    ));

    // Bunny Detective
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(DETECTIVE_PATH)),
    ));

    println!("Controls:");
    println!("  - spacebar: jump");
}

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the `walk` animation and add the `OnLanded` event.
fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    bones: Res<BoneTargets>,
    graphs: Res<Assets<AnimationGraph>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        let graph = graphs.get(&animations.graph).unwrap();

        let jump_node = graph.get(animations.jump).unwrap();
        let jump_clip = jump_node
            .clip
            .as_ref()
            .and_then(|id| clips.get_mut(id))
            .unwrap();

        // The jump height in the original animation is not very impressive :(
        // make it a little bit higher.
        jump_clip.add_curve_to_target(
            bones.root,
            TranslationCurve(
                AnimatableKeyframeCurve::new([
                    (0.0, Vec3::ZERO),
                    (0.3, Vec3::Y * 0.5),
                    (0.5, Vec3::ZERO),
                ])
                .unwrap(),
            ),
        );

        jump_clip.add_event(0.0, OnJumped);
        // Trigger the OnLanded event after 0.53s in the jump animation.
        // The feet hits the ground between frame 8 and 9 in the animation
        // and the frame rate of the animation is 16 fps: 8.5 / 16 = 5.3
        jump_clip.add_event(0.53, OnLanded);

        let walk_node = graph.get(animations.walk).unwrap();
        let walk_clip = walk_node
            .clip
            .as_ref()
            .and_then(|id| clips.get_mut(id))
            .unwrap();

        // Trigger OnStep events targeting the feet.
        walk_clip.add_event_to_target(bones.left_foot, 0.05, OnStep::Start);
        walk_clip.add_event_to_target(bones.left_foot, 0.5, OnStep::End);
        walk_clip.add_event_to_target(bones.right_foot, 0.55, OnStep::Start);
        walk_clip.add_event_to_target(bones.right_foot, 0.0, OnStep::End);

        let mut transitions = AnimationTransitions::new();

        // Make sure to start the animation via the `AnimationTransitions`
        // component. The `AnimationTransitions` component wants to manage all
        // the animations and will get confused if the animations are started
        // directly via the `AnimationPlayer`.
        transitions
            .play(&mut player, animations.walk, Duration::ZERO)
            .repeat();

        commands
            .entity(entity)
            .insert(animations.graph.clone())
            .insert(transitions);
    }
}

// Play the `jump` animation when the space bar is pressed.
fn jump_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    animations: Res<Animations>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (mut player, mut transitions) in &mut players {
        if keyboard_input.just_pressed(KeyCode::Space) {
            transitions.play(&mut player, animations.jump, Duration::from_millis(75));
        }
    }
}

fn simulate_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform, &mut Particle)>,
    time: Res<Time>,
) {
    for (entity, mut transform, mut particle) in &mut query {
        if particle.lifeteime.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        } else {
            transform.translation += particle.velocity * time.delta_seconds();
            transform.scale = Vec3::splat(particle.size.lerp(0.0, particle.lifeteime.fraction()));
            particle
                .velocity
                .smooth_nudge(&Vec3::ZERO, 4.0, time.delta_seconds());
        }
        if transform.scale.length_squared() < 0.01 {}
    }
}

fn spawn_particle<M: Material>(
    mesh: Handle<Mesh>,
    material: Handle<M>,
    translation: Vec3,
    lifetime: f32,
    size: f32,
    velocity: Vec3,
) -> impl Command {
    move |world: &mut World| {
        world.spawn((
            Particle {
                lifeteime: Timer::from_seconds(lifetime, TimerMode::Once),
                size,
                velocity,
            },
            Mesh3d(mesh),
            MeshMaterial3d(material),
            Transform {
                translation,
                scale: Vec3::splat(size),
                ..Default::default()
            },
        ));
    }
}
