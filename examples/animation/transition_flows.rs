//! Demonstrates how to crossfade animations of different body parts
//! independently, using flows of [`AnimationTransitions`].

use std::{collections::HashSet, f32::consts::PI, time::Duration};

use bevy::{
    animation::AnimationTargetId, light::CascadeShadowConfigBuilder, prelude::*,
    world_serialization::WorldInstanceReady,
};

const FOX_PATH: &str = "models/animated/Fox.glb";

// The mask groups of the fox. The head group contains the neck and head bones,
// and the body group contains every other bone.
const MASK_GROUP_HEAD: u32 = 0;
const MASK_GROUP_BODY: u32 = 1;

// The bones of the head mask group, as paths from the root of the model.
const HEAD_BONES: [&str; 2] = [
    "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03/b_Neck_04",
    "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03/b_Neck_04/b_Head_05",
];

// Each flow transitions independently of the others. The body flow
// crossfades between the clips masked to the body, and the head flow
// crossfades between the clips masked to the head.
const FLOW_BODY: usize = 0;
const FLOW_HEAD: usize = 1;

const CLIP_NAMES: [&str; 3] = ["Run", "Walk", "Survey"];

fn main() {
    App::new()
        .insert_resource(GlobalAmbientLight {
            color: Color::WHITE,
            brightness: 2000.,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            spawn_fox_asset_when_ready.run_if(not(resource_exists::<Animations>)),
        )
        .add_systems(
            Update,
            (keyboard_control, update_text).run_if(resource_exists::<Animations>),
        )
        .run();
}

#[derive(Resource)]
struct Fox(Handle<Gltf>);

#[derive(Resource)]
struct Animations {
    body: [AnimationNodeIndex; CLIP_NAMES.len()],
    head: [AnimationNodeIndex; CLIP_NAMES.len()],
    graph_handle: Handle<AnimationGraph>,
}

impl Animations {
    fn clip_name(&self, node: AnimationNodeIndex) -> &'static str {
        self.body
            .iter()
            .chain(&self.head)
            .position(|&clip| clip == node)
            .map_or("None", |index| CLIP_NAMES[index % CLIP_NAMES.len()])
    }
}

#[derive(Component)]
struct FlowsText;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.insert_resource(Fox(asset_server.load(FOX_PATH)));

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
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 200.0,
            maximum_distance: 400.0,
            ..default()
        }
        .build(),
    ));

    // Instructions and the current clip of each flow
    commands.spawn((
        FlowsText,
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn spawn_fox_asset_when_ready(
    mut commands: Commands,
    fox_handle: Res<Fox>,
    asset_server: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    clips: Res<Assets<AnimationClip>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    if !asset_server.is_loaded_with_dependencies(&fox_handle.0) {
        return;
    }

    let fox = gltfs
        .get(&fox_handle.0)
        .expect("a loaded asset should exist in the glTF assets collection");

    // Build the animation graph. Every clip is added twice: once masked to
    // only animate the body, and once masked to only animate the head.
    let mut graph = AnimationGraph::new();
    let [body, head] = [1 << MASK_GROUP_HEAD, 1 << MASK_GROUP_BODY].map(|mask| {
        CLIP_NAMES.map(|name| {
            graph.add_clip_with_mask(fox.named_animations[name].clone(), mask, 1.0, graph.root)
        })
    });

    // Add every bone animated by the clips to either the head or the body mask
    // group.
    let head_bones: HashSet<_> = HEAD_BONES
        .iter()
        .map(|path| {
            let names: Vec<_> = path.split('/').map(Name::new).collect();
            AnimationTargetId::from_names(names.iter())
        })
        .collect();
    for name in CLIP_NAMES {
        let clip = clips
            .get(&fox.named_animations[name])
            .expect("the clips are loaded along with the glTF file");
        for &target in clip.curves().keys() {
            let mask_group = if head_bones.contains(&target) {
                MASK_GROUP_HEAD
            } else {
                MASK_GROUP_BODY
            };
            graph.add_target_to_mask_group(target, mask_group);
        }
    }

    commands.insert_resource(Animations {
        body,
        head,
        graph_handle: graphs.add(graph),
    });

    // Fox
    commands
        .spawn(WorldAssetRoot(
            fox.default_scene
                .clone()
                .expect("a default scene exists in this file"),
        ))
        .observe(setup_scene);
}

fn setup_scene(
    _ready: On<WorldInstanceReady>,
    mut commands: Commands,
    animations: Res<Animations>,
    player: Single<(Entity, &mut AnimationPlayer)>,
) {
    let (entity, mut player) = player.into_inner();

    // Create one flow for the body and one for the head, and start both of
    // them walking. Make sure to start the animations via the
    // `AnimationTransitions` component. The `AnimationTransitions` component
    // wants to manage all the animations and will get confused if the
    // animations are started directly via the `AnimationPlayer`.
    let mut transitions = AnimationTransitions::new_with_flow(2.try_into().unwrap());
    transitions
        .transition_flows_unchecked(&mut player, animations.body[1], FLOW_BODY, Duration::ZERO)
        .repeat();
    transitions
        .transition_flows_unchecked(&mut player, animations.head[1], FLOW_HEAD, Duration::ZERO)
        .repeat();

    commands.entity(entity).insert((
        AnimationGraphHandle(animations.graph_handle.clone()),
        transitions,
    ));
}

fn keyboard_control(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut animation_players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
    animations: Res<Animations>,
) {
    const BODY_KEYS: [KeyCode; 3] = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3];
    const HEAD_KEYS: [KeyCode; 3] = [KeyCode::KeyQ, KeyCode::KeyW, KeyCode::KeyE];

    for (mut player, mut transitions) in &mut animation_players {
        for (flow, keys, clips) in [
            (FLOW_BODY, BODY_KEYS, animations.body),
            (FLOW_HEAD, HEAD_KEYS, animations.head),
        ] {
            for (key, clip) in keys.into_iter().zip(clips) {
                // Transitioning one flow doesn't affect the other one, so the
                // body and the head can be crossfaded at the same time.
                if keyboard_input.just_pressed(key) && transitions.current_node(flow) != Some(clip)
                {
                    transitions
                        .transition_flows_unchecked(
                            &mut player,
                            clip,
                            flow,
                            Duration::from_secs_f32(0.5),
                        )
                        .repeat();
                }
            }
        }
    }
}

fn update_text(
    animations: Res<Animations>,
    transitions: Query<&AnimationTransitions>,
    mut text: Single<&mut Text, With<FlowsText>>,
) {
    let Ok(transitions) = transitions.single() else {
        return;
    };

    let [body, head] = [FLOW_BODY, FLOW_HEAD].map(|flow| {
        transitions
            .current_node(flow)
            .map_or("None", |node| animations.clip_name(node))
    });

    text.0 = format!(
        "1 / 2 / 3: body plays Run / Walk / Survey\n\
         Q / W / E: head plays Run / Walk / Survey\n\
         \n\
         Body: {body}\n\
         Head: {head}",
    );
}
