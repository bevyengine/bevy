//! Demonstrates how to use masks to limit the scope of animations.

use bevy::{
    animation::{AnimationTarget, AnimationTargetId},
    color::palettes::css::{LIGHT_GRAY, WHITE},
    prelude::*,
};
use std::collections::HashSet;

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

// The width in pixels of the small buttons that allow the user to toggle a mask
// group on or off.
const MASK_GROUP_BUTTON_WIDTH: f32 = 250.0;

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

#[derive(Clone, Copy, Component)]
struct AnimationControl {
    // The ID of the mask group that this button controls.
    group_id: u32,
    label: AnimationLabel,
}

#[derive(Clone, Copy, Component, PartialEq, Debug)]
enum AnimationLabel {
    Idle = 0,
    Walk = 1,
    Run = 2,
    Off = 3,
}

#[derive(Clone, Debug, Resource)]
struct AnimationNodes([AnimationNodeIndex; 3]);

#[derive(Clone, Copy, Debug, Resource)]
struct AppState([MaskGroupState; 6]);

#[derive(Clone, Copy, Debug)]
struct MaskGroupState {
    clip: u8,
}

// The application entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Animation Masks Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_scene, setup_ui))
        .add_systems(Update, setup_animation_graph_once_loaded)
        .add_systems(Update, handle_button_toggles)
        .add_systems(Update, update_ui)
        .insert_resource(AmbientLight {
            color: WHITE.into(),
            brightness: 100.0,
            ..default()
        })
        .init_resource::<AppState>()
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
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(-4.0, 8.0, 13.0),
    ));

    // Spawn the fox.
    commands.spawn((
        SceneRoot(
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
    commands.spawn((
        Text::new("Click on a button to toggle animations for its associated bones"),
        Node {
            position_type: PositionType::Absolute,
            left: px(12),
            top: px(12),
            ..default()
        },
    ));

    // Add the buttons that allow the user to toggle mask groups on and off.
    commands
        .spawn(Node {
            flex_direction: FlexDirection::Column,
            position_type: PositionType::Absolute,
            row_gap: px(6),
            left: px(12),
            bottom: px(12),
            ..default()
        })
        .with_children(|parent| {
            let row_node = Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(6),
                ..default()
            };

            add_mask_group_control(parent, "Head", auto(), MASK_GROUP_HEAD);

            parent.spawn(row_node.clone()).with_children(|parent| {
                add_mask_group_control(
                    parent,
                    "Left Front Leg",
                    px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_LEFT_FRONT_LEG,
                );
                add_mask_group_control(
                    parent,
                    "Right Front Leg",
                    px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_RIGHT_FRONT_LEG,
                );
            });

            parent.spawn(row_node).with_children(|parent| {
                add_mask_group_control(
                    parent,
                    "Left Hind Leg",
                    px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_LEFT_HIND_LEG,
                );
                add_mask_group_control(
                    parent,
                    "Right Hind Leg",
                    px(MASK_GROUP_BUTTON_WIDTH),
                    MASK_GROUP_RIGHT_HIND_LEG,
                );
            });

            add_mask_group_control(parent, "Tail", auto(), MASK_GROUP_TAIL);
        });
}

// Adds a button that allows the user to toggle a mask group on and off.
//
// The button will automatically become a child of the parent that owns the
// given `ChildSpawnerCommands`.
fn add_mask_group_control(
    parent: &mut ChildSpawnerCommands,
    label: &str,
    width: Val,
    mask_group_id: u32,
) {
    let button_text_style = (
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor::WHITE,
    );
    let selected_button_text_style = (button_text_style.0.clone(), TextColor::BLACK);
    let label_text_style = (
        button_text_style.0.clone(),
        TextColor(Color::Srgba(LIGHT_GRAY)),
    );

    parent
        .spawn((
            Node {
                border: UiRect::all(px(1)),
                width,
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::ZERO,
                margin: UiRect::ZERO,
                ..default()
            },
            BorderColor::all(Color::WHITE),
            BorderRadius::all(px(3)),
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|builder| {
            builder
                .spawn((
                    Node {
                        border: UiRect::ZERO,
                        width: percent(100),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        padding: UiRect::ZERO,
                        margin: UiRect::ZERO,
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                ))
                .with_child((
                    Text::new(label),
                    label_text_style.clone(),
                    Node {
                        margin: UiRect::vertical(px(3)),
                        ..default()
                    },
                ));

            builder
                .spawn((
                    Node {
                        width: percent(100),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        border: UiRect::top(px(1)),
                        ..default()
                    },
                    BorderColor::all(Color::WHITE),
                ))
                .with_children(|builder| {
                    for (index, label) in [
                        AnimationLabel::Run,
                        AnimationLabel::Walk,
                        AnimationLabel::Idle,
                        AnimationLabel::Off,
                    ]
                    .iter()
                    .enumerate()
                    {
                        builder
                            .spawn((
                                Button,
                                BackgroundColor(if index > 0 {
                                    Color::BLACK
                                } else {
                                    Color::WHITE
                                }),
                                Node {
                                    flex_grow: 1.0,
                                    border: if index > 0 {
                                        UiRect::left(px(1))
                                    } else {
                                        UiRect::ZERO
                                    },
                                    ..default()
                                },
                                BorderColor::all(Color::WHITE),
                                AnimationControl {
                                    group_id: mask_group_id,
                                    label: *label,
                                },
                            ))
                            .with_child((
                                Text(format!("{label:?}")),
                                if index > 0 {
                                    button_text_style.clone()
                                } else {
                                    selected_button_text_style.clone()
                                },
                                TextLayout::new_with_justify(Justify::Center),
                                Node {
                                    flex_grow: 1.0,
                                    margin: UiRect::vertical(px(3)),
                                    ..default()
                                },
                            ));
                    }
                });
        });
}

// Builds up the animation graph, including the mask groups, and adds it to the
// entity with the `AnimationPlayer` that the glTF loader created.
fn setup_animation_graph_once_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    targets: Query<(Entity, &AnimationTarget)>,
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
            if !all_animation_target_ids.contains(&target.id) {
                commands.entity(target_entity).remove::<AnimationTarget>();
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

// A system that handles requests from the user to toggle mask groups on and
// off.
fn handle_button_toggles(
    mut interactions: Query<(&Interaction, &mut AnimationControl), Changed<Interaction>>,
    mut animation_players: Query<&AnimationGraphHandle, With<AnimationPlayer>>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut animation_nodes: Option<ResMut<AnimationNodes>>,
    mut app_state: ResMut<AppState>,
) {
    let Some(ref mut animation_nodes) = animation_nodes else {
        return;
    };

    for (interaction, animation_control) in interactions.iter_mut() {
        // We only care about press events.
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Toggle the state of the clip.
        app_state.0[animation_control.group_id as usize].clip = animation_control.label as u8;

        // Now grab the animation player. (There's only one in our case, but we
        // iterate just for clarity's sake.)
        for animation_graph_handle in animation_players.iter_mut() {
            // The animation graph needs to have loaded.
            let Some(animation_graph) = animation_graphs.get_mut(animation_graph_handle) else {
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
}

// A system that updates the UI based on the current app state.
fn update_ui(
    mut animation_controls: Query<(&AnimationControl, &mut BackgroundColor, &Children)>,
    texts: Query<Entity, With<Text>>,
    mut writer: TextUiWriter,
    app_state: Res<AppState>,
) {
    for (animation_control, mut background_color, kids) in animation_controls.iter_mut() {
        let enabled =
            app_state.0[animation_control.group_id as usize].clip == animation_control.label as u8;

        *background_color = if enabled {
            BackgroundColor(Color::WHITE)
        } else {
            BackgroundColor(Color::BLACK)
        };

        for &kid in kids {
            let Ok(text) = texts.get(kid) else {
                continue;
            };

            writer.for_each_color(text, |mut color| {
                color.0 = if enabled { Color::BLACK } else { Color::WHITE };
            });
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        AppState([MaskGroupState { clip: 0 }; 6])
    }
}
