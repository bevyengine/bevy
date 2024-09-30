//! Plays animations from a skinned glTF.

use std::{f32::consts::PI, time::Duration};

use bevy::{
    animation::{
        animate_targets_and_trigger_events,
        animation_event::{AnimationEvent, ReflectAnimationEvent},
        AnimationTargetId, RepeatAnimation,
    },
    color::palettes::css::WHITE,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
};

const FOX_PATH: &str = "models/animated/Fox.glb";

fn main() {
    App::new()
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
        })
        .add_plugins(DefaultPlugins)
        .init_resource::<FoxFeetIds>()
        .init_resource::<FoxStep>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            setup_scene_once_loaded.before(animate_targets_and_trigger_events),
        )
        .add_systems(Update, keyboard_animation_control)
        .add_systems(Update, update_particle)
        .observe(FoxStep::observer)
        .register_type::<FoxStep>()
        .run();
}

#[derive(Resource)]
struct Animations {
    animations: Vec<AnimationNodeIndex>,
    #[allow(dead_code)]
    graph: Handle<AnimationGraph>,
}

#[derive(Resource)]
struct FoxFeetIds {
    forward_right: AnimationTargetId,
    forward_left: AnimationTargetId,
    back_right: AnimationTargetId,
    back_left: AnimationTargetId,
}

impl Default for FoxFeetIds {
    fn default() -> Self {
        Self {
            forward_right: AnimationTargetId::from_iter([
                "root",
                "_rootJoint",
                "b_Root_00",
                "b_Hip_01",
                "b_Spine01_02",
                "b_Spine02_03",
                "b_RightUpperArm_06",
                "b_RightForeArm_07",
                "b_RightHand_08",
            ]),
            forward_left: AnimationTargetId::from_iter([
                "root",
                "_rootJoint",
                "b_Root_00",
                "b_Hip_01",
                "b_Spine01_02",
                "b_Spine02_03",
                "b_LeftUpperArm_09",
                "b_LeftForeArm_010",
                "b_LeftHand_011",
            ]),
            back_right: AnimationTargetId::from_iter([
                "root",
                "_rootJoint",
                "b_Root_00",
                "b_Hip_01",
                "b_RightLeg01_019",
                "b_RightLeg02_020",
                "b_RightFoot01_021",
                "b_RightFoot02_022",
            ]),
            back_left: AnimationTargetId::from_iter([
                "root",
                "_rootJoint",
                "b_Root_00",
                "b_Hip_01",
                "b_LeftLeg01_015",
                "b_LeftLeg02_016",
                "b_LeftFoot01_017",
                "b_LeftFoot02_018",
            ]),
        }
    }
}

#[derive(Component, Debug)]
struct Particle {
    size: f32,
    lifetime: Timer,
}

#[derive(Resource, Event, Reflect, Clone)]
#[reflect(AnimationEvent)]
struct FoxStep {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for FoxStep {
    fn from_world(world: &mut World) -> Self {
        let mesh = world.resource_mut::<Assets<Mesh>>().add(Sphere::new(10.0));
        let material = world
            .resource_mut::<Assets<StandardMaterial>>()
            .add(StandardMaterial::from_color(WHITE));
        Self { mesh, material }
    }
}

impl FoxStep {
    fn observer(trigger: Trigger<Self>, query: Query<&GlobalTransform>, mut commands: Commands) {
        let transform = query.get(trigger.entity()).unwrap().compute_transform();
        commands.spawn((
            Particle {
                lifetime: Timer::from_seconds(1.0, TimerMode::Once),
                size: 1.0,
            },
            MaterialMeshBundle {
                mesh: trigger.event().mesh.clone(),
                material: trigger.event().material.clone(),
                transform: Transform {
                    translation: transform.translation.reject_from(Vec3::Y),
                    scale: Vec3::splat(1.0),
                    ..Default::default()
                },
                ..Default::default()
            },
        ));
        // println!("STEP: {}", transform.translation);
    }
}

impl AnimationEvent for FoxStep {
    fn trigger(&self, entity: Entity, world: &mut World) {
        world.entity_mut(entity).trigger(self.clone());
    }

    fn clone_value(&self) -> Box<dyn AnimationEvent> {
        Box::new(self.clone())
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
        asset_server.load(GltfAssetLabel::Animation(2).from_asset(FOX_PATH)),
        asset_server.load(GltfAssetLabel::Animation(1).from_asset(FOX_PATH)),
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(FOX_PATH)),
    ]);

    // Insert a resource with the current scene information
    let graph_handle = graphs.add(graph);
    commands.insert_resource(Animations {
        animations: node_indices,
        graph: graph_handle,
    });

    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(100.0, 100.0, 150.0)
            .looking_at(Vec3::new(0.0, 20.0, 0.0), Vec3::Y),
        ..default()
    });

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(500000.0, 500000.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // Light
    commands.spawn(DirectionalLightBundle {
        transform: Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, 1.0, -PI / 4.)),
        directional_light: DirectionalLight {
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
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset(FOX_PATH)),
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

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the animation.
fn setup_scene_once_loaded(
    mut commands: Commands,
    animations: Res<Animations>,
    feet: Res<FoxFeetIds>,
    step: Res<FoxStep>,
    mut clips: ResMut<Assets<AnimationClip>>,
    graphs: Res<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        let graph = graphs.get(&animations.graph).unwrap();
        let node = graph.get(animations.animations[0]).unwrap();
        let clip = clips.get_mut(node.clip.as_ref().unwrap()).unwrap();
        clip.add_event_with_id(feet.forward_right, 0.46, step.clone());
        clip.add_event_with_id(feet.forward_left, 0.64, step.clone());
        clip.add_event_with_id(feet.back_right, 0.14, step.clone());
        clip.add_event_with_id(feet.back_left, 0.02, step.clone());

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
            .insert(animations.graph.clone())
            .insert(transitions);
    }
}

fn keyboard_animation_control(
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

        if keyboard_input.just_pressed(KeyCode::Digit3) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(3))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::Digit5) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation
                .set_repeat(RepeatAnimation::Count(5))
                .replay();
        }

        if keyboard_input.just_pressed(KeyCode::KeyL) {
            let playing_animation = player.animation_mut(playing_animation_index).unwrap();
            playing_animation.set_repeat(RepeatAnimation::Forever);
        }
    }
}

fn update_particle(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Particle, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut particle, mut transform) in &mut query {
        if particle.lifetime.tick(time.delta()).just_finished() {
            commands.entity(entity).despawn();
        }
        transform.scale = Vec3::ONE * particle.size.lerp(0.0, particle.lifetime.fraction());
    }
}
