//! Demonstrates animation blending with animation graphs.

use bevy::{
    color::palettes::{
        basic::WHITE,
        css::{ANTIQUE_WHITE, DARK_GREEN},
    },
    prelude::{Color::Srgba, *},
};

#[derive(Debug)]
struct NodeRect {
    left: f32,
    bottom: f32,
    width: f32,
    height: f32,
}

struct Line {
    left: f32,
    bottom: f32,
    length: f32,
}

#[derive(Resource)]
struct AppAssets {
    animation_graph: Handle<AnimationGraph>,
    node_indices: [AnimationNodeIndex; 3],
}

#[derive(Resource)]
struct AnimationWeights {
    weights: [f32; 3],
}

enum NodeType {
    Clip(ClipNodeText),
    Other(&'static str),
}

#[derive(Clone, Component)]
struct ClipNodeText {
    text: &'static str,
    index: usize,
}

#[derive(Resource, Default)]
enum DragState {
    #[default]
    NotDragging,
    Dragging {
        weight_index: usize,
        reference_x_pos: f32,
        reference_weight: f32,
    },
}

const WEIGHT_ADJUST_SPEED: f32 = 0.01;

static HELP_TEXT: &str = "Click and drag an animation clip node to change its weight";

static NODE_TYPES: [NodeType; 5] = [
    NodeType::Clip(ClipNodeText::new("Idle", 0)),
    NodeType::Clip(ClipNodeText::new("Walk", 1)),
    NodeType::Other("Root"),
    NodeType::Other("Blend\n0.5"),
    NodeType::Clip(ClipNodeText::new("Run", 2)),
];

static NODE_RECTS: [NodeRect; 5] = [
    NodeRect::new(10.00, 10.00, 97.64, 48.41),
    NodeRect::new(10.00, 78.41, 97.64, 48.41),
    NodeRect::new(286.08, 78.41, 97.64, 48.41),
    NodeRect::new(148.04, 44.20, 97.64, 48.41),
    NodeRect::new(10.00, 146.82, 97.64, 48.41),
];

static HORIZONTAL_LINES: [Line; 6] = [
    Line::new(107.64, 34.21, 20.20),
    Line::new(107.64, 102.61, 20.20),
    Line::new(107.64, 171.02, 158.24),
    Line::new(127.84, 68.41, 20.20),
    Line::new(245.68, 68.41, 20.20),
    Line::new(265.88, 102.61, 20.20),
];

static VERTICAL_LINES: [Line; 2] = [
    Line::new(127.83, 34.21, 68.40),
    Line::new(265.88, 68.41, 102.61),
];

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

    let animation_graph = animation_graphs.add(animation_graph);

    commands.insert_resource(AppAssets {
        animation_graph,
        node_indices,
    });


    commands.spawn(SceneBundle {
        scene: asset_server.load("models/animated/Fox.glb#Scene0"),
        transform: Transform::from_scale(Vec3::splat(0.05)),
        ..default()
    });

    setup_camera_and_light(&mut commands);

    setup_help_text(&mut commands, &mut asset_server);
    setup_node_rects(&mut commands, &mut asset_server);
    setup_node_lines(&mut commands);
}

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

fn setup_node_rects(commands: &mut Commands, asset_server: &mut AssetServer) {
    for (node_rect, node_text) in NODE_RECTS.iter().zip(NODE_TYPES.iter()) {
        let node_string = match *node_text {
            NodeType::Clip(ref clip) => clip.text,
            NodeType::Other(text) => text,
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

fn handle_weight_drag(
    windows: Query<&Window>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut drag_state: ResMut<DragState>,
    mut animation_weights: ResMut<AnimationWeights>,
) {
    for window in windows.iter() {
        let Some(cursor_pos) = window.cursor_position() else {
            continue;
        };

        if mouse_input.just_pressed(MouseButton::Left) {
            for (node_rect, node_type) in NODE_RECTS.iter().zip(NODE_TYPES.iter()) {
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
                        weight_index,
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
            weight_index,
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

fn update_ui(
    mut text_query: Query<(&mut Text, &ClipNodeText)>,
    mut node_query: Query<(&mut Style, &ClipNodeText), (With<Node>, Without<Text>)>,
    animation_weights: Res<AnimationWeights>,
) {
    for (mut text, clip_node_text) in text_query.iter_mut() {
        text.sections[0].value = format!(
            "{}\n{}",
            clip_node_text.text, animation_weights.weights[clip_node_text.index]
        );
    }

    for (mut style, clip_node_text) in node_query.iter_mut() {
        // All nodes are the same width, so `NODE_RECTS[0]` is as good as any other.
        style.width =
            Val::Px(NODE_RECTS[0].width * animation_weights.weights[clip_node_text.index]);
    }
}

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
            if !animation_player.animation_is_playing(animation_node_index) {
                animation_player.play(animation_node_index);
            }
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
    const fn new(text: &'static str, index: usize) -> Self {
        Self { text, index }
    }
}

impl NodeRect {
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
    const fn new(left: f32, bottom: f32, length: f32) -> Self {
        Self {
            left,
            bottom,
            length,
        }
    }
}
