//! Demonstrates how to use masks to limit the scope of animations.

use bevy::{animation::AnimationTargetId, color::palettes::css::WHITE, prelude::*};

// IDs of the mask groups we define for the running fox model.
//
// Each mask group defines a set of bones for which animations can be toggled on
// and off.
const MASK_GROUP_LEFT_FRONT_LEG: u32 = 0;
const MASK_GROUP_RIGHT_FRONT_LEG: u32 = 1;
const MASK_GROUP_LEFT_HIND_LEG: u32 = 2;
const MASK_GROUP_RIGHT_HIND_LEG: u32 = 3;
const MASK_GROUP_TAIL: u32 = 4;

// The width in pixels of the small buttons that allow the user to toggle a mask
// group on or off.
const MASK_GROUP_SMALL_BUTTON_WIDTH: f32 = 150.0;

// The ID of the animation in the glTF file that we're going to play.
const FOX_RUN_ANIMATION: usize = 2;

// The names of the bones that each mask group consists of. Each mask group is
// defined as a (prefix, suffix) tuple. The mask group consists of a single
// bone chain rooted at the prefix. For example, if the chain's prefix is
// "A/B/C" and the suffix is "D/E", then the bones that will be included in the
// mask group are "A/B/C", "A/B/C/D", and "A/B/C/D/E".
//
// The fact that our mask groups are single chains of bones isn't anything
// specific to Bevy; it just so happens to be the case for the model we're
// using. A mask group can consist of any set of animation targets, regardless
// of whether they form a single chain.
const MASK_GROUP_PATHS: [(&str, &str); 5] = [
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

// A component that identifies a clickable button that allows the user to toggle
// a mask group on or off.
#[derive(Component)]
struct MaskGroupControl {
    // The ID of the mask group that this button controls.
    group_id: u32,

    // Whether animations are playing for this mask group.
    //
    // Note that this is the opposite of the `mask` field in `AnimationGraph`:
    // i.e. it's true if the group is *not* presently masked, and false if the
    // group *is* masked.
    enabled: bool,
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
        .insert_resource(AmbientLight {
            color: WHITE.into(),
            brightness: 100.0,
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
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-15.0, 10.0, 20.0)
            .looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });

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
        TextNEW::new("Click on a button to toggle animations for its associated bones"),
        Style {
            position_type: PositionType::Absolute,
            left: Val::Px(12.0),
            top: Val::Px(12.0),
            ..default()
        },
    ));

    // Add the buttons that allow the user to toggle mask groups on and off.
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                position_type: PositionType::Absolute,
                row_gap: Val::Px(6.0),
                left: Val::Px(12.0),
                bottom: Val::Px(12.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            let row_style = Style {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                ..default()
            };

            parent
                .spawn(NodeBundle {
                    style: row_style.clone(),
                    ..default()
                })
                .with_children(|parent| {
                    add_mask_group_control(
                        parent,
                        "Left Front Leg",
                        Val::Px(MASK_GROUP_SMALL_BUTTON_WIDTH),
                        MASK_GROUP_LEFT_FRONT_LEG,
                    );
                    add_mask_group_control(
                        parent,
                        "Right Front Leg",
                        Val::Px(MASK_GROUP_SMALL_BUTTON_WIDTH),
                        MASK_GROUP_RIGHT_FRONT_LEG,
                    );
                });

            parent
                .spawn(NodeBundle {
                    style: row_style,
                    ..default()
                })
                .with_children(|parent| {
                    add_mask_group_control(
                        parent,
                        "Left Hind Leg",
                        Val::Px(MASK_GROUP_SMALL_BUTTON_WIDTH),
                        MASK_GROUP_LEFT_HIND_LEG,
                    );
                    add_mask_group_control(
                        parent,
                        "Right Hind Leg",
                        Val::Px(MASK_GROUP_SMALL_BUTTON_WIDTH),
                        MASK_GROUP_RIGHT_HIND_LEG,
                    );
                });

            add_mask_group_control(parent, "Tail", Val::Auto, MASK_GROUP_TAIL);
        });
}

// Adds a button that allows the user to toggle a mask group on and off.
//
// The button will automatically become a child of the parent that owns the
// given `ChildBuilder`.
fn add_mask_group_control(parent: &mut ChildBuilder, label: &str, width: Val, mask_group_id: u32) {
    parent
        .spawn(ButtonBundle {
            style: Style {
                border: UiRect::all(Val::Px(1.0)),
                width,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                padding: UiRect::all(Val::Px(6.0)),
                margin: UiRect::ZERO,
                ..default()
            },
            border_color: BorderColor(Color::WHITE),
            border_radius: BorderRadius::all(Val::Px(3.0)),
            background_color: Color::WHITE.into(),
            ..default()
        })
        .insert(MaskGroupControl {
            group_id: mask_group_id,
            enabled: true,
        })
        .with_child((
            TextNEW::new(label),
            TextStyle {
                font_size: 14.0,
                color: Color::BLACK,
                ..default()
            },
        ));
}

// Builds up the animation graph, including the mask groups, and adds it to the
// entity with the `AnimationPlayer` that the glTF loader created.
fn setup_animation_graph_once_loaded(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    mut players: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
) {
    for (entity, mut player) in &mut players {
        // Load the animation clip from the glTF file.
        let (mut animation_graph, node_index) = AnimationGraph::from_clip(asset_server.load(
            GltfAssetLabel::Animation(FOX_RUN_ANIMATION).from_asset("models/animated/Fox.glb"),
        ));

        // Create each mask group.
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
            }
        }

        // We're doing constructing the animation graph. Add it as an asset.
        let animation_graph = animation_graphs.add(animation_graph);
        commands.entity(entity).insert(animation_graph);

        // Finally, play the animation.
        player.play(node_index).repeat();
    }
}

// A system that handles requests from the user to toggle mask groups on and
// off.
fn handle_button_toggles(
    mut interactions: Query<
        (
            &Interaction,
            &mut MaskGroupControl,
            &mut BackgroundColor,
            &Children,
        ),
        Changed<Interaction>,
    >,
    mut writer: UiTextWriter,
    mut animation_players: Query<(&Handle<AnimationGraph>, &AnimationPlayer)>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (interaction, mut mask_group_control, mut button_background_color, children) in
        interactions.iter_mut()
    {
        // We only care about press events.
        if *interaction != Interaction::Pressed {
            continue;
        }

        // Toggle the state of the mask.
        mask_group_control.enabled = !mask_group_control.enabled;

        // Update the background color of the button.
        button_background_color.0 = if mask_group_control.enabled {
            Color::WHITE
        } else {
            Color::BLACK
        };

        // Update the text color of the button.
        for &kid in children.iter() {
            writer.for_each(kid, |_, _, _, mut style| {
                style.color = if mask_group_control.enabled {
                    Color::BLACK
                } else {
                    Color::WHITE
                };
                true
            });
        }

        // Now grab the animation player. (There's only one in our case, but we
        // iterate just for clarity's sake.)
        for (animation_graph_handle, animation_player) in animation_players.iter_mut() {
            // The animation graph needs to have loaded.
            let Some(animation_graph) = animation_graphs.get_mut(animation_graph_handle) else {
                continue;
            };

            // Grab the animation graph node that's currently playing.
            let Some((&animation_node_index, _)) = animation_player.playing_animations().next()
            else {
                continue;
            };
            let Some(animation_node) = animation_graph.get_mut(animation_node_index) else {
                continue;
            };

            // Enable or disable the mask group as appropriate.
            if mask_group_control.enabled {
                animation_node.mask &= !(1 << mask_group_control.group_id);
            } else {
                animation_node.mask |= 1 << mask_group_control.group_id;
            }
        }
    }
}
