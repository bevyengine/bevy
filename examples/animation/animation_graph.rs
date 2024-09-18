//! Demonstrates animation blending with animation graphs.
//!
//! The animation graph is shown on screen. You can change the weights of the
//! playing animations by clicking and dragging left or right within the nodes.

#[cfg(not(target_arch = "wasm32"))]
use std::{fs::File, path::Path};

use bevy::{
    animation::animate_targets,
    color::palettes::{
        basic::WHITE,
        css::{ANTIQUE_WHITE, DARK_GREEN},
    },
    prelude::*,
    ui::RelativeCursorPosition,
};

use argh::FromArgs;
#[cfg(not(target_arch = "wasm32"))]
use bevy::asset::io::file::FileAssetReader;
#[cfg(not(target_arch = "wasm32"))]
use bevy::tasks::IoTaskPool;
#[cfg(not(target_arch = "wasm32"))]
use ron::ser::PrettyConfig;

/// Where to find the serialized animation graph.
static ANIMATION_GRAPH_PATH: &str = "animation_graphs/Fox.animgraph.ron";

/// The indices of the nodes containing animation clips in the graph.
static CLIP_NODE_INDICES: [u32; 3] = [2, 3, 4];

/// The help text in the upper left corner.
static HELP_TEXT: &str = "Click and drag an animation clip node to change its weight
Click the checkbox to toggle between additive and shared mode";

/// The node widgets in the UI.
static NODE_TYPES: [NodeType; 5] = [
    NodeType::Clip(ClipNode::new("Idle", 0)),
    NodeType::Clip(ClipNode::new("Walk", 1)),
    NodeType::Blend("Root"),
    NodeType::Blend("Blend\n0.5"),
    NodeType::Clip(ClipNode::new("Run", 2)),
];

/// The positions of the node widgets in the UI.
///
/// These are in the same order as [`NODE_TYPES`] above.
static NODE_RECTS: [NodeRect; 5] = [
    NodeRect::new(10.00, 10.00, 97.64, 48.41),
    NodeRect::new(10.00, 78.41, 97.64, 48.41),
    NodeRect::new(286.08, 78.41, 97.64, 48.41),
    NodeRect::new(148.04, 44.20, 97.64, 48.41),
    NodeRect::new(10.00, 146.82, 97.64, 48.41),
];

/// The positions of the horizontal lines in the UI.
static HORIZONTAL_LINES: [Line; 6] = [
    Line::new(107.64, 34.21, 20.20),
    Line::new(107.64, 102.61, 20.20),
    Line::new(107.64, 171.02, 158.24),
    Line::new(127.84, 68.41, 20.20),
    Line::new(245.68, 68.41, 20.20),
    Line::new(265.88, 102.61, 20.20),
];

/// The positions of the vertical lines in the UI.
static VERTICAL_LINES: [Line; 2] = [
    Line::new(127.83, 34.21, 68.40),
    Line::new(265.88, 68.41, 102.61),
];

/// Initializes the app.
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Animation Graph Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, (setup_assets, setup_scene, setup_ui))
        .add_systems(Update, init_animations.before(animate_targets))
        .add_systems(
            Update,
            (
                handle_weight_drag,
                handle_additive_toggle,
                update_ui,
                sync_weights,
                sync_additive_mode,
            )
                .chain(),
        )
        .insert_resource(args)
        .insert_resource(AmbientLight {
            color: WHITE.into(),
            brightness: 100.0,
        })
        .run();
}

/// Demonstrates animation blending with animation graphs
#[derive(FromArgs, Resource)]
struct Args {
    /// disables loading of the animation graph asset from disk
    #[argh(switch)]
    no_load: bool,
    /// regenerates the asset file; implies `--no-load`
    #[argh(switch)]
    save: bool,
}

/// The [`AnimationGraph`] asset, which specifies how the animations are to
/// be blended together.
#[derive(Clone, Resource)]
struct ExampleAnimationGraph(Handle<AnimationGraph>);

/// The current states of the three playing animations.
#[derive(Component)]
struct ExampleAnimationStates([ExampleAnimationState; 3]);

/// The current state of a single animation clip node.
#[derive(Clone, Copy)]
struct ExampleAnimationState {
    /// Weight of the clip node.
    weight: f32,
    /// Whether this node is additive or not.
    additive: bool,
}

/// Initializes the scene.
fn setup_assets(
    mut commands: Commands,
    mut asset_server: ResMut<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    args: Res<Args>,
) {
    // Create or load the assets.
    if args.no_load || args.save {
        setup_assets_programmatically(
            &mut commands,
            &mut asset_server,
            &mut animation_graphs,
            args.save,
        );
    } else {
        setup_assets_via_serialized_animation_graph(&mut commands, &mut asset_server);
    }
}

fn setup_ui(mut commands: Commands) {
    setup_help_text(&mut commands);
    setup_node_rects(&mut commands);
    setup_node_lines(&mut commands);
}

/// Creates the assets programmatically, including the animation graph.
/// Optionally saves them to disk if `save` is present (corresponding to the
/// `--save` option).
fn setup_assets_programmatically(
    commands: &mut Commands,
    asset_server: &mut AssetServer,
    animation_graphs: &mut Assets<AnimationGraph>,
    _save: bool,
) {
    // Create the nodes.
    let mut animation_graph = AnimationGraph::new();
    let blend_node = animation_graph.add_blend(0.5, animation_graph.root);
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(0).from_asset("models/animated/Fox.glb")),
        1.0,
        animation_graph.root,
    );
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(1).from_asset("models/animated/Fox.glb")),
        1.0,
        blend_node,
    );
    animation_graph.add_clip(
        asset_server.load(GltfAssetLabel::Animation(2).from_asset("models/animated/Fox.glb")),
        1.0,
        blend_node,
    );

    // If asked to save, do so.
    #[cfg(not(target_arch = "wasm32"))]
    if _save {
        let animation_graph = animation_graph.clone();

        IoTaskPool::get()
            .spawn(async move {
                let mut animation_graph_writer = File::create(Path::join(
                    &FileAssetReader::get_base_path(),
                    Path::join(Path::new("assets"), Path::new(ANIMATION_GRAPH_PATH)),
                ))
                .expect("Failed to open the animation graph asset");
                ron::ser::to_writer_pretty(
                    &mut animation_graph_writer,
                    &animation_graph,
                    PrettyConfig::default(),
                )
                .expect("Failed to serialize the animation graph");
            })
            .detach();
    }

    // Add the graph.
    let handle = animation_graphs.add(animation_graph);

    // Save the assets in a resource.
    commands.insert_resource(ExampleAnimationGraph(handle));
}

fn setup_assets_via_serialized_animation_graph(
    commands: &mut Commands,
    asset_server: &mut AssetServer,
) {
    commands.insert_resource(ExampleAnimationGraph(
        asset_server.load(ANIMATION_GRAPH_PATH),
    ));
}

/// Spawns the animated fox.
fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10.0, 5.0, 13.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 10_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-4.0, 8.0, 13.0),
        ..default()
    });

    commands.spawn(SceneBundle {
        scene: asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb")),
        transform: Transform::from_scale(Vec3::splat(0.07)),
        ..default()
    });

    // Ground

    commands.spawn(PbrBundle {
        mesh: meshes.add(Circle::new(7.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        transform: Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        ..default()
    });
}

/// Places the help text at the top left of the window.
fn setup_help_text(commands: &mut Commands) {
    commands.spawn(TextBundle {
        text: Text::from_section(HELP_TEXT, TextStyle::default()),
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        ..default()
    });
}

/// Initializes the node UI widgets.
fn setup_node_rects(commands: &mut Commands) {
    for (node_rect, node_type) in NODE_RECTS.iter().zip(NODE_TYPES.iter()) {
        let node_string = match *node_type {
            NodeType::Clip(ref clip) => clip.text,
            NodeType::Blend(text) => text,
        };

        let text = commands
            .spawn(TextBundle {
                text: Text::from_section(
                    node_string,
                    TextStyle {
                        font_size: 16.0,
                        color: ANTIQUE_WHITE.into(),
                        ..default()
                    },
                )
                .with_justify(JustifyText::Center),
                ..default()
            })
            .id();

        let container = {
            let mut container = commands.spawn((
                NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(node_rect.bottom),
                        left: Val::Px(node_rect.left),
                        height: Val::Px(node_rect.height),
                        width: Val::Px(node_rect.width),
                        align_items: AlignItems::Center,
                        justify_items: JustifyItems::Center,
                        align_content: AlignContent::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    border_color: WHITE.into(),
                    ..default()
                },
                Outline::new(Val::Px(1.), Val::ZERO, Color::WHITE),
            ));

            if let NodeType::Clip(ref clip) = node_type {
                container = container.insert((
                    Interaction::None,
                    RelativeCursorPosition::default(),
                    (*clip).clone(),
                ));
            }

            container.id()
        };

        if let NodeType::Clip(ref clip) = node_type {
            // Create the background color.
            let background = commands
                .spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        top: Val::Px(0.),
                        left: Val::Px(0.),
                        height: Val::Px(node_rect.height),
                        width: Val::Px(node_rect.width),
                        ..default()
                    },
                    background_color: DARK_GREEN.into(),
                    ..default()
                })
                .id();

            commands.entity(container).add_child(background);

            // Create the additive toggle checkbox.
            let additive_toggle = commands
                .spawn((
                    NodeBundle {
                        style: Style {
                            position_type: PositionType::Absolute,
                            bottom: Val::Px(5.),
                            left: Val::Px(5.),
                            height: Val::Px(20.),
                            width: Val::Px(20.),
                            ..default()
                        },
                        ..default()
                    },
                    Outline::new(Val::Px(1.), Val::ZERO, Color::WHITE),
                    Interaction::None,
                    clip.clone(),
                    AdditiveModeCheckbox,
                ))
                .id();

            commands.entity(container).add_child(additive_toggle);
        }

        commands.entity(container).add_child(text);
    }
}

/// Creates boxes for the horizontal and vertical lines.
///
/// This is a bit hacky: it uses 1-pixel-wide and 1-pixel-high boxes to draw
/// vertical and horizontal lines, respectively.
fn setup_node_lines(commands: &mut Commands) {
    for line in &HORIZONTAL_LINES {
        commands.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(line.bottom),
                left: Val::Px(line.left),
                height: Val::Px(0.0),
                width: Val::Px(line.length),
                border: UiRect::bottom(Val::Px(1.0)),
                ..default()
            },
            border_color: WHITE.into(),
            ..default()
        });
    }

    for line in &VERTICAL_LINES {
        commands.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(line.bottom),
                left: Val::Px(line.left),
                height: Val::Px(line.length),
                width: Val::Px(0.0),
                border: UiRect::left(Val::Px(1.0)),
                ..default()
            },
            border_color: WHITE.into(),
            ..default()
        });
    }
}

/// Attaches the animation graph to the scene, and plays all three animations.
fn init_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AnimationPlayer)>,
    animation_graph: Res<ExampleAnimationGraph>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }

    for (entity, mut player) in query.iter_mut() {
        commands
            .entity(entity)
            .insert((animation_graph.0.clone(), ExampleAnimationStates::default()));
        for &node_index in &CLIP_NODE_INDICES {
            player.play(node_index.into()).repeat();
        }

        *done = true;
    }
}

/// Read cursor position relative to clip nodes, allowing the user to change weights
/// when dragging the node UI widgets.
fn handle_weight_drag(
    interaction_query: Query<
        (&Interaction, &RelativeCursorPosition, &ClipNode),
        Without<AdditiveModeCheckbox>,
    >,
    mut animation_states_query: Query<&mut ExampleAnimationStates>,
) {
    for (interaction, relative_cursor, clip_node) in &interaction_query {
        if !matches!(*interaction, Interaction::Pressed) {
            continue;
        }

        let Some(pos) = relative_cursor.normalized else {
            continue;
        };

        for mut animation_states in animation_states_query.iter_mut() {
            animation_states.0[clip_node.index].weight = pos.x.clamp(0., 1.);
        }
    }
}

/// Reads interactions with the additive mode check boxes, allowing the user to
/// change whether a specific node is additive or not.
fn handle_additive_toggle(
    interaction_query: Query<
        (&Interaction, &ClipNode),
        (Changed<Interaction>, With<AdditiveModeCheckbox>),
    >,
    mut animation_states_query: Query<&mut ExampleAnimationStates>,
) {
    for (interaction, clip_node) in &interaction_query {
        if !matches!(*interaction, Interaction::Pressed) {
            continue;
        }

        for mut animation_states in animation_states_query.iter_mut() {
            let state = &mut animation_states.0[clip_node.index];
            state.additive = !state.additive;
        }
    }
}

// Updates the UI based on the weights that the user has chosen.
fn update_ui(
    mut text_query: Query<&mut Text>,
    mut background_query: Query<&mut Style, Without<Text>>,
    mut additive_toggle_query: Query<&mut BackgroundColor, (Without<Text>, With<ClipNode>)>,
    container_query: Query<(&Children, &ClipNode)>,
    animation_weights_query: Query<&ExampleAnimationStates, Changed<ExampleAnimationStates>>,
) {
    for animation_weights in animation_weights_query.iter() {
        for (children, clip_node) in &container_query {
            // Draw the green background color to visually indicate the weight.
            let mut bg_iter = background_query.iter_many_mut(children);
            if let Some(mut style) = bg_iter.fetch_next() {
                // All nodes are the same width, so `NODE_RECTS[0]` is as good as any other.
                style.width =
                    Val::Px(NODE_RECTS[0].width * animation_weights.0[clip_node.index].weight);
            }

            // Update the background of the additive checkbox.
            let mut additive_toggle_iter = additive_toggle_query.iter_many_mut(children);
            if let Some(mut bg_color) = additive_toggle_iter.fetch_next() {
                *bg_color = if animation_weights.0[clip_node.index].additive {
                    WHITE.into()
                } else {
                    Color::NONE.into()
                };
            }

            // Update the node labels with the current weights.
            let mut text_iter = text_query.iter_many_mut(children);
            if let Some(mut text) = text_iter.fetch_next() {
                text.sections[0].value = format!(
                    "{}\n{:.2}",
                    clip_node.text, animation_weights.0[clip_node.index].weight
                );
            }
        }
    }
}

/// Takes the weights that were set in the UI and assigns them to the actual
/// playing animation.
fn sync_weights(mut query: Query<(&mut AnimationPlayer, &ExampleAnimationStates)>) {
    for (mut animation_player, animation_states) in query.iter_mut() {
        for (&animation_node_index, animation_weight) in CLIP_NODE_INDICES
            .iter()
            .zip(animation_states.0.iter().map(|state| state.weight))
        {
            // If the animation happens to be no longer active, restart it.
            if !animation_player.animation_is_playing(animation_node_index.into()) {
                animation_player.play(animation_node_index.into());
            }

            // Set the weight.
            if let Some(active_animation) =
                animation_player.animation_mut(animation_node_index.into())
            {
                active_animation.set_weight(animation_weight);
            }
        }
    }
}

/// Takes the state of the additive checkboxes in the UI and assigns them to the
/// actual nodes in the animation graph.
fn sync_additive_mode(
    query: Query<(&Handle<AnimationGraph>, &ExampleAnimationStates)>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    for (animation_graph_handle, animation_states) in query.iter() {
        let Some(animation_graph) = animation_graphs.get_mut(animation_graph_handle) else {
            continue;
        };

        for (&animation_node_index, additive) in CLIP_NODE_INDICES
            .iter()
            .zip(animation_states.0.iter().map(|state| state.additive))
        {
            let animation_node_index = AnimationNodeIndex::from(animation_node_index);
            let animation_node = animation_graph.get_mut(animation_node_index).unwrap();

            // Set the additive mode of this node.
            animation_node.additive = additive;
        }
    }
}

/// An on-screen representation of a node.
#[derive(Debug)]
struct NodeRect {
    /// The number of pixels that this rectangle is from the left edge of the
    /// window.
    left: f32,
    /// The number of pixels that this rectangle is from the bottom edge of the
    /// window.
    bottom: f32,
    /// The width of this rectangle in pixels.
    width: f32,
    /// The height of this rectangle in pixels.
    height: f32,
}

/// Either a straight horizontal or a straight vertical line on screen.
///
/// The line starts at (`left`, `bottom`) and goes either right (if the line is
/// horizontal) or down (if the line is vertical).
struct Line {
    /// The number of pixels that the start of this line is from the left edge
    /// of the screen.
    left: f32,
    /// The number of pixels that the start of this line is from the bottom edge
    /// of the screen.
    bottom: f32,
    /// The length of the line.
    length: f32,
}

/// The type of each node in the UI: either a clip node or a blend node.
enum NodeType {
    /// A clip node, which specifies an animation.
    Clip(ClipNode),
    /// A blend node with no animation and a string label.
    Blend(&'static str),
}

/// The label for the UI representation of a clip node.
#[derive(Clone, Component)]
struct ClipNode {
    /// The string label of the node.
    text: &'static str,
    /// Which of the three animations this UI widget represents.
    index: usize,
}

/// Marker component for UI nodes which represent the additive toggle check box.
#[derive(Component)]
struct AdditiveModeCheckbox;

impl Default for ExampleAnimationStates {
    fn default() -> Self {
        Self(
            [ExampleAnimationState {
                weight: 1.0,
                additive: false,
            }; 3],
        )
    }
}

impl ClipNode {
    /// Creates a new [`ClipNodeText`] from a label and the animation index.
    const fn new(text: &'static str, index: usize) -> Self {
        Self { text, index }
    }
}

impl NodeRect {
    /// Creates a new [`NodeRect`] from the lower-left corner and size.
    ///
    /// Note that node rectangles are anchored in the *lower*-left corner. The
    /// `bottom` parameter specifies vertical distance from the *bottom* of the
    /// window.
    const fn new(left: f32, bottom: f32, width: f32, height: f32) -> NodeRect {
        NodeRect {
            left,
            bottom,
            width,
            height,
        }
    }
}

impl Line {
    /// Creates a new [`Line`], either horizontal or vertical.
    ///
    /// Note that the line's start point is anchored in the lower-*left* corner,
    /// and that the `length` extends either to the right or downward.
    const fn new(left: f32, bottom: f32, length: f32) -> Self {
        Self {
            left,
            bottom,
            length,
        }
    }
}
