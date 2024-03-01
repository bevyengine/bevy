//! Demonstrates animation blending with animation graphs.
//!
//! The animation graph is shown on screen. You can change the weights of the
//! playing animations by clicking and dragging left or right within the nodes.

use bevy::{
    color::palettes::{
        basic::WHITE,
        css::{ANTIQUE_WHITE, DARK_GREEN},
    },
    prelude::{Color::Srgba, *},
};

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

/// Assets that the example needs to use.
#[derive(Resource)]
struct AppAssets {
    /// The [`AnimationGraph`] asset, which specifies how the animations are to
    /// be blended together.
    animation_graph: Handle<AnimationGraph>,
    /// Indices of the three animations in the graph.
    ///
    /// This value is populated when the graph is loaded.
    node_indices: [AnimationNodeIndex; 3],
}

/// The current weights of the three playing animations.
#[derive(Resource)]
struct AnimationWeights {
    /// The weights of the three playing animations.
    weights: [f32; 3],
}

/// The type of each node in the UI: either a clip node or a blend node.
enum NodeType {
    /// A clip node, which specifies an animation.
    Clip(ClipNodeText),
    /// A blend node with no animation and a string label.
    Blend(&'static str),
}

/// The label for the UI representation of a clip node.
#[derive(Clone, Component)]
struct ClipNodeText {
    /// The string label of the node.
    text: &'static str,
    /// Which of the three animations this UI widget represents.
    index: usize,
}

/// Whether, and where, the mouse is dragging.
#[derive(Resource, Default)]
enum DragState {
    /// The mouse isn't being dragged.
    #[default]
    NotDragging,
    /// The mouse is being dragged.
    Dragging {
        /// Which animation the mouse is dragging.
        clip_index: usize,
        /// The X position of the mouse when the drag started.
        reference_x_pos: f32,
        /// The weight that the dragged node had when the drag started.
        reference_weight: f32,
    },
}

/// How much weight a drag of one mouse pixel corresponds to.
const WEIGHT_ADJUST_SPEED: f32 = 0.01;

/// The help text in the upper left corner.
static HELP_TEXT: &str = "Click and drag an animation clip node to change its weight";

/// The node widgets in the UI.
static NODE_TYPES: [NodeType; 5] = [
    NodeType::Clip(ClipNodeText::new("Idle", 0)),
    NodeType::Clip(ClipNodeText::new("Walk", 1)),
    NodeType::Blend("Root"),
    NodeType::Blend("Blend\n0.5"),
    NodeType::Clip(ClipNodeText::new("Run", 2)),
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
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Animation Graph Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(PreUpdate, init_animations)
        .add_systems(
            Update,
            (handle_weight_drag, update_ui, sync_weights).chain(),
        )
        .init_resource::<AnimationWeights>()
        .init_resource::<DragState>()
        .insert_resource(AmbientLight {
            color: Srgba(WHITE),
            brightness: 100.0,
        })
        .run();
}

fn setup(
    mut commands: Commands,
    mut asset_server: ResMut<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
) {
    // Create the assets.
    setup_assets(&mut commands, &mut asset_server, &mut animation_graphs);

    // Create the scene.
    setup_scene(&mut commands, &mut asset_server);
    setup_camera_and_light(&mut commands);

    // Create the UI.
    setup_help_text(&mut commands, &mut asset_server);
    setup_node_rects(&mut commands, &mut asset_server);
    setup_node_lines(&mut commands);
}

/// Creates the assets, including the animation graph.
fn setup_assets(
    commands: &mut Commands,
    asset_server: &mut AssetServer,
    animation_graphs: &mut Assets<AnimationGraph>,
) {
    // Create the nodes.
    let mut animation_graph = AnimationGraph::new();
    let blend_node = animation_graph.add_blend(0.5, animation_graph.root);
    let node_indices: [_; 3] = [
        animation_graph.add_clip(
            asset_server.load("models/animated/Fox.glb#Animation0"),
            1.0,
            animation_graph.root,
        ),
        animation_graph.add_clip(
            asset_server.load("models/animated/Fox.glb#Animation1"),
            1.0,
            blend_node,
        ),
        animation_graph.add_clip(
            asset_server.load("models/animated/Fox.glb#Animation2"),
            1.0,
            blend_node,
        ),
    ];

    // Add the graph.
    let animation_graph = animation_graphs.add(animation_graph);

    // Save the assets in a resource.
    commands.insert_resource(AppAssets {
        animation_graph,
        node_indices,
    });
}

/// Spawns the animated fox.
fn setup_scene(commands: &mut Commands, asset_server: &mut AssetServer) {
    commands.spawn(SceneBundle {
        scene: asset_server.load("models/animated/Fox.glb#Scene0"),
        transform: Transform::from_scale(Vec3::splat(0.05)),
        ..default()
    });
}

/// Spawns the camera and point light.
fn setup_camera_and_light(commands: &mut Commands) {
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-10.0, 5.0, 13.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 10000000.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(-4.0, 6.0, 13.0),
        ..default()
    });
}

/// Places the help text at the top left of the window.
fn setup_help_text(commands: &mut Commands, asset_server: &mut AssetServer) {
    commands.spawn(TextBundle {
        text: Text::from_section(
            HELP_TEXT,
            TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: 24.0,
                color: Srgba(ANTIQUE_WHITE),
            },
        ),
        style: Style {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            left: Val::Px(10.0),
            ..default()
        },
        ..default()
    });
}

/// Initializes the node UI widgets.
fn setup_node_rects(commands: &mut Commands, asset_server: &mut AssetServer) {
    for (node_rect, node_text) in NODE_RECTS.iter().zip(NODE_TYPES.iter()) {
        let node_string = match *node_text {
            NodeType::Clip(ref clip) => clip.text,
            NodeType::Blend(text) => text,
        };

        let mut text = commands.spawn(TextBundle {
            text: Text::from_section(
                node_string,
                TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: 16.0,
                    color: Srgba(ANTIQUE_WHITE),
                },
            )
            .with_justify(JustifyText::Center),
            style: Style::default(),
            ..default()
        });

        if let NodeType::Clip(ref clip) = node_text {
            text.insert((*clip).clone());
        }
        let text = text.id();

        // Create the background color.
        if let NodeType::Clip(ref clip) = node_text {
            commands
                .spawn(NodeBundle {
                    style: Style {
                        position_type: PositionType::Absolute,
                        bottom: Val::Px(node_rect.bottom),
                        left: Val::Px(node_rect.left),
                        height: Val::Px(node_rect.height),
                        width: Val::Px(node_rect.width),
                        ..default()
                    },
                    background_color: BackgroundColor(Srgba(DARK_GREEN)),
                    ..default()
                })
                .insert((*clip).clone());
        }

        commands
            .spawn(NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    bottom: Val::Px(node_rect.bottom),
                    left: Val::Px(node_rect.left),
                    height: Val::Px(node_rect.height),
                    width: Val::Px(node_rect.width),
                    border: UiRect::all(Val::Px(1.0)),
                    align_items: AlignItems::Center,
                    justify_items: JustifyItems::Center,
                    align_self: AlignSelf::Center,
                    justify_self: JustifySelf::Center,
                    align_content: AlignContent::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                border_color: BorderColor(Srgba(WHITE)),
                ..default()
            })
            .add_child(text);
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
            border_color: BorderColor(Srgba(WHITE)),
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
            border_color: BorderColor(Srgba(WHITE)),
            ..default()
        });
    }
}

/// Attaches the animation graph to the scene, and plays all three animations.
fn init_animations(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AnimationPlayer), Added<AnimationPlayer>>,
    app_assets: Res<AppAssets>,
) {
    for (entity, mut player) in query.iter_mut() {
        commands
            .entity(entity)
            .insert(app_assets.animation_graph.clone());
        for node_index in app_assets.node_indices {
            player.play(node_index).repeat();
        }
    }
}

/// Processes mouse events, allowing the user to change weights when dragging
/// the node UI widgets.
fn handle_weight_drag(
    windows: Query<&Window>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<DragState>,
    mut animation_weights: ResMut<AnimationWeights>,
) {
    for window in windows.iter() {
        // If the cursor is outside the window, bail.
        let Some(cursor_pos) = window.cursor_position() else {
            continue;
        };

        // Handle the start of a drag.
        if mouse_input.just_pressed(MouseButton::Left) {
            for (node_rect, node_type) in NODE_RECTS.iter().zip(NODE_TYPES.iter()) {
                // Ignore clicks on non-clip nodes.
                let NodeType::Clip(ClipNodeText {
                    index: weight_index,
                    ..
                }) = *node_type
                else {
                    continue;
                };

                if cursor_pos.x >= node_rect.left
                    && cursor_pos.x < node_rect.left + node_rect.width
                    && cursor_pos.y < window.height() - node_rect.bottom
                    && cursor_pos.y >= window.height() - node_rect.bottom - node_rect.height
                {
                    *drag_state = DragState::Dragging {
                        clip_index: weight_index,
                        reference_x_pos: cursor_pos.x,
                        reference_weight: animation_weights.weights[weight_index],
                    }
                }
            }
        }

        if mouse_input.just_released(MouseButton::Left) {
            *drag_state = DragState::NotDragging;
        }

        let DragState::Dragging {
            clip_index: weight_index,
            reference_x_pos,
            reference_weight,
        } = *drag_state
        else {
            continue;
        };

        animation_weights.weights[weight_index] = (reference_weight
            + WEIGHT_ADJUST_SPEED * (cursor_pos.x - reference_x_pos))
            .clamp(0.0, 1.0);
    }
}

// Updates the UI based on the weights that the user has chosen.
fn update_ui(
    mut text_query: Query<(&mut Text, &ClipNodeText)>,
    mut node_query: Query<(&mut Style, &ClipNodeText), (With<Node>, Without<Text>)>,
    animation_weights: Res<AnimationWeights>,
) {
    // Update the node labels with the current weights.
    for (mut text, clip_node_text) in text_query.iter_mut() {
        text.sections[0].value = format!(
            "{}\n{}",
            clip_node_text.text, animation_weights.weights[clip_node_text.index]
        );
    }

    // Draw the green background color to visually indicate the weight.
    for (mut style, clip_node_text) in node_query.iter_mut() {
        // All nodes are the same width, so `NODE_RECTS[0]` is as good as any other.
        style.width =
            Val::Px(NODE_RECTS[0].width * animation_weights.weights[clip_node_text.index]);
    }
}

/// Takes the weights that were set in the UI and assigns them to the actual
/// playing animation.
fn sync_weights(
    mut query: Query<&mut AnimationPlayer>,
    app_assets: Res<AppAssets>,
    animation_weights: Res<AnimationWeights>,
) {
    for mut animation_player in query.iter_mut() {
        for (&animation_node_index, &animation_weight) in app_assets
            .node_indices
            .iter()
            .zip(animation_weights.weights.iter())
        {
            // If the animation happens to be no longer active, restart it.
            if !animation_player.animation_is_playing(animation_node_index) {
                animation_player.play(animation_node_index);
            }

            // Set the weight.
            if let Some(active_animation) = animation_player.animation_mut(animation_node_index) {
                active_animation.set_weight(animation_weight);
            }
        }
    }
}

impl Default for AnimationWeights {
    fn default() -> Self {
        Self { weights: [1.0; 3] }
    }
}

impl ClipNodeText {
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
