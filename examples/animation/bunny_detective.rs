//! Demonstrate how to use animation events with an animated character.

use std::{f32::consts::PI, time::Duration};

use bevy::{
    animation::{
        animation_event::{AnimationEvent, ReflectAnimationEvent},
        AnimationTargetId,
    },
    color::palettes::css::WHITE,
    pbr::CascadeShadowConfigBuilder,
    prelude::*,
};
use rand::{thread_rng, Rng};

const DETECTIVE_PATH: &str = "models/animated/buntective.glb";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(AmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
        })
        .init_resource::<OnLanded>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (setup_scene_once_loaded, jump_input, simulate_particles),
        )
        .observe(OnLanded::observer)
        .run();
}

#[derive(Resource)]
struct Animations {
    walk: AnimationNodeIndex,
    jump: AnimationNodeIndex,
    graph: Handle<AnimationGraph>,
}

#[derive(Component)]
struct Particle {
    lifeteime: Timer,
    size: f32,
    velocity: Vec3,
}

// The event that will be fired once the detective hits the ground in the jump animation.
// It's also a resource to make it easier to re-use the asset handles for the mesh and material.
#[derive(Event, Resource, Reflect, Clone)]
#[reflect(AnimationEvent)]
struct OnLanded {
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
}

impl FromWorld for OnLanded {
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

impl AnimationEvent for OnLanded {
    fn trigger(&self, player: Entity, _time: f32, target: Entity, world: &mut World) {
        println!("LANDED!");
        world.entity_mut(target).trigger(self.clone());
        // Trigger the event on the animation player as well in order to transition to the walk animation at the end of the jump.
        // TODO: there might be better ways of doing this, works fine though
        world.entity_mut(player).trigger(self.clone());
    }

    fn clone_value(&self) -> Box<dyn AnimationEvent> {
        Box::new(self.clone())
    }
}

impl OnLanded {
    // The observer that will run when the detective hits the ground in the jump animation.
    fn observer(
        trigger: Trigger<Self>,
        mut commands: Commands,
        animations: Res<Animations>,
        mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
        transforms: Query<&GlobalTransform>,
    ) {
        if let Ok((mut player, mut transitions)) = players.get_mut(trigger.entity()) {
            // If this event was targeted to the animation player, transition to the walk animation.
            // FIXME: setting the `transition_duration` to `1.0s` results in the `OnLanded` event being triggered many times when the transition ends.
            // I think it's because the transition duration is then longer than the rest of the jump animation, and that causes the bug somehow.
            transitions
                .play(&mut player, animations.walk, Duration::from_secs_f32(0.5))
                .repeat();
        } else {
            // spawn a bunch of particles :3
            let translation = transforms.get(trigger.entity()).unwrap().translation();
            let mut rng = thread_rng();
            for _ in 0..25 {
                let horizontal = rng.gen::<Dir2>() * rng.gen_range(8.0..12.0);
                let vertical = rng.gen_range(0.0..4.0);
                let size = rng.gen_range(0.5..1.0);
                commands.spawn((
                    Particle {
                        lifeteime: Timer::from_seconds(1.0, TimerMode::Once),
                        size,
                        velocity: Vec3::new(horizontal.x, vertical, horizontal.y),
                    },
                    MaterialMeshBundle {
                        mesh: trigger.event().mesh.clone(),
                        material: trigger.event().material.clone(),
                        transform: Transform {
                            translation: translation.reject_from(Vec3::Y),
                            scale: Vec3::splat(size),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                ));
            }
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
        ..default()
    });

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(500000.0, 500000.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        transform: Transform::from_xyz(0.0, -0.075, 0.0),
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
            first_cascade_far_bound: 2.0,
            maximum_distance: 10.0,
            ..default()
        }
        .into(),
        ..default()
    });

    // Bunny Detective
    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset(DETECTIVE_PATH)),
        ..default()
    });

    println!("Controls:");
    println!("  - spacebar: jump");
}

// An `AnimationPlayer` is automatically added to the scene when it's ready.
// When the player is added, start the `walk` animation and add the `OnLanded` event.
fn setup_scene_once_loaded(
    mut commands: Commands,
    landed: Res<OnLanded>,
    animations: Res<Animations>,
    graphs: Res<Assets<AnimationGraph>>,
    mut clips: ResMut<Assets<AnimationClip>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        let graph = graphs.get(&animations.graph).unwrap();
        let node = graph.get(animations.jump).unwrap();
        let clip = node.clip.as_ref().and_then(|id| clips.get_mut(id)).unwrap();

        // Get the id of the "pelvis" bone, this is used to spawn the particles
        // TODO: this is not necessary in this case but I want to demonstrate the use of `add_event_to_target`
        let target_id = AnimationTargetId::from_iter(["Armature", "pelvis"]);
        // Trigger the `OnLanded` event on 0.53s in the jump animation.
        // The detective hits the ground between frame 8 and 9 and the frame rate is 16 fps.
        // 8.5 / 16 = 5.3
        clip.add_event_to_target(target_id, 0.53, landed.clone());

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
