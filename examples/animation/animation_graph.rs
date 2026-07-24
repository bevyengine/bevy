//! Demonstrates animation blending with animation graphs.
//!
//! The animation graph is shown on screen. You can change the weights of the
//! playing animations by clicking and dragging left or right within the nodes.

use bevy::{
    color::palettes::{basic::WHITE, css::DARK_GREEN},
    feathers::{
        controls::{FeathersNumberInput, NumberInputPrecision, NumberInputValue},
        dark_theme::create_dark_theme,
        display::caption,
        theme::UiTheme,
        FeathersPlugins,
    },
    prelude::*,
    ui_widgets::ValueChange,
};

use argh::FromArgs;

use crate::number_input_f32::number_input_f32;

#[cfg(not(target_arch = "wasm32"))]
use {
    bevy::{asset::io::file::FileAssetReader, tasks::IoTaskPool},
    ron::ser::PrettyConfig,
    std::{fs::File, path::Path},
};

#[path = "../helpers/number_input_f32.rs"]
mod number_input_f32;

/// Where to find the serialized animation graph.
static ANIMATION_GRAPH_PATH: &str = "animation_graphs/Fox.animgraph.ron";

/// The indices of the nodes containing animation clips in the graph.
static CLIP_NODE_INDICES: [u32; 3] = [2, 3, 4];

/// The help text in the upper left corner.
static HELP_TEXT: &str =
    "Click and drag an animation clip's number input to change its weight. Values must be between 0 and 1.";

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
    NodeRect::new(10.00, 10.00, 250., 48.41),
    NodeRect::new(10.00, 78.41, 250., 48.41),
    NodeRect::new(438.44, 78.41, 97.64, 48.41),
    NodeRect::new(300.4, 112.61, 97.64, 48.41),
    NodeRect::new(10.00, 146.82, 250., 48.41),
];

/// The positions of the horizontal lines in the UI.
static HORIZONTAL_LINES: [Line; 6] = [
    Line::new(260., 34.21, 158.24),
    Line::new(260., 102.61, 20.20),
    Line::new(260., 171.02, 20.20),
    Line::new(280.2, 136.82, 20.20),
    Line::new(398.04, 136.82, 20.20),
    Line::new(418.04, 102.61, 20.20),
];

/// The positions of the vertical lines in the UI.
static VERTICAL_LINES: [Line; 2] = [
    Line::new(280.19, 102.61, 68.40),
    Line::new(418.24, 34.21, 102.61),
];

/// Initializes the app.
fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Animation Graph Example".into(),
                    ..default()
                }),
                ..default()
            }),
            FeathersPlugins,
        ))
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, (setup_assets, setup_scene, setup_ui))
        .add_systems(Update, init_animations)
        .add_systems(Update, sync_weights)
        .add_observer(handle_weight_value_change)
        .insert_resource(args)
        .insert_resource(GlobalAmbientLight {
            color: WHITE.into(),
            brightness: 100.0,
            ..default()
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

/// The current weights of the three playing animations.
#[derive(Component)]
struct ExampleAnimationWeights {
    /// The weights of the three playing animations.
    weights: [f32; 3],
}

/// Marker component for the background of the parents of the weight number inputs.
#[derive(Component, Default, Clone)]
struct WeightBackground;

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
                use std::io::Write;

                let animation_graph: SerializedAnimationGraph = animation_graph
                    .try_into()
                    .expect("The animation graph failed to convert to its serialized form");

                let serialized_graph =
                    ron::ser::to_string_pretty(&animation_graph, PrettyConfig::default())
                        .expect("Failed to serialize the animation graph");
                let mut animation_graph_writer = File::create(Path::join(
                    &FileAssetReader::get_base_path(),
                    Path::join(Path::new("assets"), Path::new(ANIMATION_GRAPH_PATH)),
                ))
                .expect("Failed to open the animation graph asset");
                animation_graph_writer
                    .write_all(serialized_graph.as_bytes())
                    .expect("Failed to write the animation graph");
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
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-10.0, 5.0, 13.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
    ));

    commands.spawn((
        PointLight {
            intensity: 10_000_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(-4.0, 8.0, 13.0),
    ));

    commands.spawn((
        WorldAssetRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/animated/Fox.glb")),
        ),
        Transform::from_scale(Vec3::splat(0.07)),
    ));

    // Ground

    commands.spawn((
        Mesh3d(meshes.add(Circle::new(7.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
}

/// Places the help text at the top left of the window.
fn setup_help_text(commands: &mut Commands) {
    commands.spawn((
        Text::new(HELP_TEXT),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

/// Initializes the node UI widgets.
fn setup_node_rects(commands: &mut Commands) {
    let base_node_scene = |node_rect: &NodeRect| {
        bsn! {
            Node {
                position_type: PositionType::Absolute,
                bottom: px(node_rect.bottom),
                left: px(node_rect.left),
                height: px(node_rect.height),
                width: px(node_rect.width),
                align_items: AlignItems::Center,
                justify_items: JustifyItems::Center,
                align_content: AlignContent::Center,
                justify_content: JustifyContent::Center,
            }
            BorderColor::all(WHITE)
            Outline::new(px(1), Val::ZERO, Color::WHITE)
        }
    };
    for (node_rect, node_type) in NODE_RECTS.iter().zip(NODE_TYPES.iter()) {
        match node_type {
            NodeType::Clip(clip) => {
                commands.spawn_scene(bsn! {
                    base_node_scene(node_rect)
                    Children [
                        ZIndex(1)
                        number_input_f32(clip.text, Some(clip.clone()),
                            ExampleAnimationWeights::default().weights[clip.index], NumberInputPrecision(2), 0. ..1.),

                        // The background node that fills up based on the number input value.
                        WeightBackground
                        template_value(clip.clone())
                        Node {
                            position_type: PositionType::Absolute,
                            top: px(0),
                            left: px(0),
                            height: px(node_rect.height),
                            width: px(node_rect.width),
                        }
                        BackgroundColor({DARK_GREEN.with_alpha(0.5)}),
                    ]
                });
            }
            NodeType::Blend(text) => {
                commands.spawn_scene(bsn! {
                    base_node_scene(node_rect)
                    Children [
                        caption(*text)
                    ]
                });
            }
        };
    }
}

/// Creates boxes for the horizontal and vertical lines.
///
/// This is a bit hacky: it uses 1-pixel-wide and 1-pixel-high boxes to draw
/// vertical and horizontal lines, respectively.
fn setup_node_lines(commands: &mut Commands) {
    for line in &HORIZONTAL_LINES {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(line.bottom),
                left: px(line.left),
                height: px(0),
                width: px(line.length),
                border: UiRect::bottom(px(1)),
                ..default()
            },
            BorderColor::all(WHITE),
        ));
    }

    for line in &VERTICAL_LINES {
        commands.spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: px(line.bottom),
                left: px(line.left),
                height: px(line.length),
                width: px(0),
                border: UiRect::left(px(1)),
                ..default()
            },
            BorderColor::all(WHITE),
        ));
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
        commands.entity(entity).insert((
            AnimationGraphHandle(animation_graph.0.clone()),
            ExampleAnimationWeights::default(),
        ));
        for &node_index in &CLIP_NODE_INDICES {
            player.play(node_index.into()).repeat();
        }

        *done = true;
    }
}

/// Read the change in weight from the input values and update accordingly.
fn handle_weight_value_change(
    value_change: On<ValueChange<f32>>,
    number_input_q: Query<&ClipNode, With<FeathersNumberInput>>,
    mut weight_background_q: Query<(&mut Node, &ClipNode), With<WeightBackground>>,
    mut animation_weights_query: Query<&mut ExampleAnimationWeights>,

    mut commands: Commands,
) {
    let Ok(clip_node) = number_input_q.get(value_change.source) else {
        return;
    };

    for mut animation_weights in animation_weights_query.iter_mut() {
        animation_weights.weights[clip_node.index] = value_change.value;
    }

    commands
        .entity(value_change.source)
        .insert(NumberInputValue::F32(value_change.value));

    // Draw the green background color to visually indicate the weight.
    for (mut node, weight_clip_node) in weight_background_q.iter_mut() {
        if weight_clip_node.index == clip_node.index {
            // All weight nodes are the same width, so `NODE_RECTS[0]` is as good as any other.
            node.width = px(NODE_RECTS[0].width * value_change.value);
        }
    }
}

/// Takes the weights that were set in the UI and assigns them to the actual
/// playing animation.
fn sync_weights(mut query: Query<(&mut AnimationPlayer, &ExampleAnimationWeights)>) {
    for (mut animation_player, animation_weights) in query.iter_mut() {
        for (&animation_node_index, &animation_weight) in CLIP_NODE_INDICES
            .iter()
            .zip(animation_weights.weights.iter())
        {
            // If the animation happens to be no longer active, restart it.
            if !animation_player.is_playing_animation(animation_node_index.into()) {
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
#[derive(Clone, Component, Default)]
struct ClipNode {
    /// The string label of the node.
    text: &'static str,
    /// Which of the three animations this UI widget represents.
    index: usize,
}

impl Default for ExampleAnimationWeights {
    fn default() -> Self {
        Self { weights: [1.0; 3] }
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
