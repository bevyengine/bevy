//! Demonstrates how to use masks to limit the scope of animations.

use crate::radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};
use bevy::{
    animation::{AnimatedBy, AnimationTargetId},
    color::palettes::css::WHITE,
    feathers::{dark_theme::create_dark_theme, display::label, theme::UiTheme, FeathersPlugins},
    prelude::*,
    ui_widgets::{radio_self_update, ValueChange},
};
use std::collections::HashSet;

#[path = "../helpers/radio.rs"]
mod radio;

// IDs of the mask groups we define for the running fox model.
//
// Each mask group defines a set of bones for which animations can be toggled on
// and off.
const MASK_GROUP_HEAD: u32 = 0;
const MASK_GROUP_LEFT_FRONT_LEG: u32 = 1;
const MASK_GROUP_RIGHT_FRONT_LEG: u32 = 2;
const MASK_GROUP_LEFT_HIND_LEG: u32 = 3;
const MASK_GROUP_RIGHT_HIND_LEG: u32 = 4;
const MASK_GROUP_TAIL: u32 = 5;

// The names of the bones that each mask group consists of. Each mask group is
// defined as a (prefix, suffix) tuple. The mask group consists of a single
// bone chain rooted at the prefix. For example, if the chain's prefix is
// "A/B/C" and the suffix is "D/E", then the bones that will be included in the
// mask group are "A/B/C", "A/B/C/D", and "A/B/C/D/E".
//
// The fact that our mask groups are single chains of bones isn't an engine
// requirement; it just so happens to be the case for the model we're using. A
// mask group can consist of any set of animation targets, regardless of whether
// they form a single chain.
const MASK_GROUP_PATHS: [(&str, &str); 6] = [
    // Head
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03",
        "b_Neck_04/b_Head_05",
    ),
    // Left front leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03/b_LeftUpperArm_09",
        "b_LeftForeArm_010/b_LeftHand_011",
    ),
    // Right front leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Spine01_02/b_Spine02_03/b_RightUpperArm_06",
        "b_RightForeArm_07/b_RightHand_08",
    ),
    // Left hind leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_LeftLeg01_015",
        "b_LeftLeg02_016/b_LeftFoot01_017/b_LeftFoot02_018",
    ),
    // Right hind leg
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_RightLeg01_019",
        "b_RightLeg02_020/b_RightFoot01_021/b_RightFoot02_022",
    ),
    // Tail
    (
        "root/_rootJoint/b_Root_00/b_Hip_01/b_Tail01_012",
        "b_Tail02_013/b_Tail03_014",
    ),
];

/// Identifies an animation for a specific mask group that the user
/// can select.
#[derive(Clone, Copy, Component, Default)]
struct AnimationControl {
    // The ID of the mask group that this button controls.
    group_id: u32,
    label: AnimationLabel,
}

impl AnimationControl {
    fn new(group_id: u32, label: AnimationLabel) -> Self {
        Self { group_id, label }
    }
}

/// The four types of animations per mask group
#[derive(Clone, Copy, Component, PartialEq, Debug, Default)]
enum AnimationLabel {
    #[default]
    Idle = 0,
    Walk = 1,
    Run = 2,
    Off = 3,
}

impl AnimationLabel {
    fn label(&self) -> &'static str {
        match self {
            Self::Idle => "Idle",
            Self::Walk => "Walk",
            Self::Run => "Run",
            Self::Off => "Off",
        }
    }
}

#[derive(Clone, Debug, Resource)]
struct AnimationNodes([AnimationNodeIndex; 3]);

// The application entry point.
fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Animation Masks Example".into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(Update, setup_animation_graph_once_loaded)
        .add_observer(handle_animation_control_change)
        .add_observer(radio_self_update)
        .insert_resource(GlobalAmbientLight {
            color: WHITE.into(),
            brightness: 100.0,
            ..default()
        })
        .run();
}

// Spawns the 3D objects in the scene, and loads the fox animation from the glTF
// file.
fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the camera.
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-15.0, 10.0, 20.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    // Spawn the light.
    commands.spawn((
        PointLight {
            intensity: 10_000_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-4.0, 8.0, 13.0),
    ));

    // Spawn the fox.
    commands.spawn((
        WorldAssetRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb")),
        ),
        Transform::from_scale(Vec3::splat(0.07)),
    ));

    // Spawn the ground.
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(7.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
}

// Creates the UI.
fn setup_ui(mut commands: Commands) {
    // Add help text.
    commands.spawn_scene(bsn! {
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
        }
        Children [
            Text::new("Click on a button to toggle animations for its associated bones")
        ]
    });

    // Add the buttons that allow the user to toggle mask groups on and off.
    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        Node {
            align_items: AlignItems::Center,
        }
        Children [
            feathers_option_buttons("Head", &make_animation_controls(MASK_GROUP_HEAD), 2),

            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
            }
            Children [
                feathers_option_buttons("Left Front Leg", &make_animation_controls(MASK_GROUP_LEFT_FRONT_LEG), 2),
                label(" / "),
                feathers_option_buttons("Right Front Leg", &make_animation_controls(MASK_GROUP_RIGHT_FRONT_LEG), 2),
            ],

            Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
            }
            Children [
                feathers_option_buttons("Left Hind Leg", &make_animation_controls(MASK_GROUP_LEFT_HIND_LEG), 2),
                label(" / "),
                feathers_option_buttons("Right Hind Leg", &make_animation_controls(MASK_GROUP_RIGHT_HIND_LEG), 2),
            ],

            feathers_option_buttons("Tail", &make_animation_controls(MASK_GROUP_TAIL), 2),
        ]
    });
}

// Makes the Radio Button Options for a given animation group.
fn make_animation_controls(group_id: u32) -> [(AnimationControl, &'static str); 4] {
    [
        (
            AnimationControl::new(group_id, AnimationLabel::Run),
            AnimationLabel::Run.label(),
        ),
        (
            AnimationControl::new(group_id, AnimationLabel::Walk),
            AnimationLabel::Walk.label(),
        ),
        (
            AnimationControl::new(group_id, AnimationLabel::Idle),
            AnimationLabel::Idle.label(),
        ),
        (
            AnimationControl::new(group_id, AnimationLabel::Off),
            AnimationLabel::Off.label(),
        ),
    ]
}

// Builds up the animation graph, including the mask groups, and adds it to the
// entity with the `AnimationPlayer` that the glTF loader created.
fn setup_animation_graph_once_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    targets: Query<(Entity, &AnimationTargetId)>,
) {
    for (entity, mut player) in &mut players {
        // Load the animation clip from the glTF file.
        let mut animation_graph = AnimationGraph::new();
        let blend_node = animation_graph.add_additive_blend(1.0, animation_graph.root);

        let animation_graph_nodes: [AnimationNodeIndex; 3] =
            std::array::from_fn(|animation_index| {
                let handle = asset_server.load(
                    GltfAssetLabel::Animation(animation_index)
                        .from_asset("models/animated/Fox.glb"),
                );
                let mask = if animation_index == 0 { 0 } else { 0x3f };
                animation_graph.add_clip_with_mask(handle, mask, 1.0, blend_node)
            });

        // Create each mask group.
        let mut all_animation_target_ids = HashSet::new();
        for (mask_group_index, (mask_group_prefix, mask_group_suffix)) in
            MASK_GROUP_PATHS.iter().enumerate()
        {
            // Split up the prefix and suffix, and convert them into `Name`s.
            let prefix: Vec<_> = mask_group_prefix.split('/').map(Name::new).collect();
            let suffix: Vec<_> = mask_group_suffix.split('/').map(Name::new).collect();

            // Add each bone in the chain to the appropriate mask group.
            for chain_length in 0..=suffix.len() {
                let animation_target_id = AnimationTargetId::from_names(
                    prefix.iter().chain(suffix[0..chain_length].iter()),
                );
                animation_graph
                    .add_target_to_mask_group(animation_target_id, mask_group_index as u32);
                all_animation_target_ids.insert(animation_target_id);
            }
        }

        // We're doing constructing the animation graph. Add it as an asset.
        let animation_graph = animation_graphs.add(animation_graph);
        commands
            .entity(entity)
            .insert(AnimationGraphHandle(animation_graph));

        // Remove animation targets that aren't in any of the mask groups. If we
        // don't do that, those bones will play all animations at once, which is
        // ugly.
        for (target_entity, target) in &targets {
            if !all_animation_target_ids.contains(target) {
                commands
                    .entity(target_entity)
                    .remove::<AnimationTargetId>()
                    .remove::<AnimatedBy>();
            }
        }

        // Play the animation.
        for animation_graph_node in animation_graph_nodes {
            player.play(animation_graph_node).repeat();
        }

        // Record the graph nodes.
        commands.insert_resource(AnimationNodes(animation_graph_nodes));
    }
}

// An observer that handles requests from the user to toggle mask groups on and
// off.
fn handle_animation_control_change(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&RadioButtonOptionValue<AnimationControl>>,
    mut animation_players: Query<&AnimationGraphHandle, With<AnimationPlayer>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_nodes: Option<ResMut<AnimationNodes>>,
) {
    let Some(ref mut animation_nodes) = animation_nodes else {
        return;
    };

    let Ok(RadioButtonOptionValue(animation_control)) = new_value_query.get(event.value) else {
        return;
    };

    // Grab the animation player. (There's only one in our case, but we
    // iterate just for clarity's sake.)
    for animation_graph_handle in animation_players.iter_mut() {
        // The animation graph needs to have loaded.
        let Some(mut animation_graph) = animation_graphs.get_mut(animation_graph_handle) else {
            continue;
        };

        for (clip_index, &animation_node_index) in animation_nodes.0.iter().enumerate() {
            let Some(animation_node) = animation_graph.get_mut(animation_node_index) else {
                continue;
            };

            if animation_control.label as usize == clip_index {
                animation_node.mask &= !(1 << animation_control.group_id);
            } else {
                animation_node.mask |= 1 << animation_control.group_id;
            }
        }
    }
}
