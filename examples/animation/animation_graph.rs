//! Demonstrates animation blending with animation graphs.

use bevy::prelude::*;

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

static NODE_RECTS: [NodeRect; 5] = [
    NodeRect {
        left: 10f32,
        bottom: 10.000250000000051,
        width: 97.63524999999998,
        height: 48.407249999999976,
    },
    NodeRect {
        left: 10f32,
        bottom: 78.40775000000002,
        width: 97.63524999999998,
        height: 48.407249999999976,
    },
    NodeRect {
        left: 286.08f32,
        bottom: 78.40775000000002,
        width: 97.63524999999998,
        height: 48.407249999999976,
    },
    NodeRect {
        left: 148.04025f32,
        bottom: 44.20275000000004,
        width: 97.63525000000001,
        height: 48.407249999999976,
    },
    NodeRect {
        left: 10f32,
        bottom: 146.81525000000005,
        width: 97.63524999999998,
        height: 48.407249999999976,
    },
];

static NODE_TYPES: [NodeType; 5] = [
    NodeType::Clip(ClipNodeText::new("Idle", 0)),
    NodeType::Clip(ClipNodeText::new("Walk", 1)),
    NodeType::Other("Root"),
    NodeType::Other("Blend\n0.5"),
    NodeType::Clip(ClipNodeText::new("Run", 2)),
];

static HORIZONTAL_LINES: [Line; 6] = [
    Line {
        left: 107.63525000000001,
        bottom: 34.20500000000004,
        length: 20.195999999999998,
    },
    Line {
        left: 107.63525000000001,
        bottom: 102.61000000000001,
        length: 20.195999999999998,
    },
    Line {
        left: 107.63525000000001,
        bottom: 171.01749999999998,
        length: 158.24224999999998,
    },
    Line {
        left: 127.83775,
        bottom: 68.40750000000003,
        length: 20.202499999999986,
    },
    Line {
        left: 245.675,
        bottom: 68.40750000000003,
        length: 20.202499999999986,
    },
    Line {
        left: 265.8775,
        bottom: 102.61000000000001,
        length: 20.202499999999986,
    },
];
static VERTICAL_LINES: [Line; 2] = [
    Line {
        left: 127.83125000000001,
        bottom: 34.20500000000004,
        length: 68.40499999999997,
    },
    Line {
        left: 265.8775,
        bottom: 68.40750000000003,
        length: 102.60999999999996,
    },
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
            color: LegacyColor::WHITE,
            brightness: 100.0,
        })
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: ResMut<AssetServer>,
    mut animation_graphs: ResMut<Assets<AnimationGraph>>,
    animation_weights: Res<AnimationWeights>,
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

    commands.spawn(SceneBundle {
        scene: asset_server.load("models/animated/Fox.glb#Scene0"),
        transform: Transform::from_scale(Vec3::splat(0.05)),
        ..default()
    });

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
                    color: LegacyColor::ANTIQUE_WHITE,
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
                    background_color: BackgroundColor(LegacyColor::DARK_GREEN),
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
                border_color: BorderColor(LegacyColor::WHITE),
                ..default()
            })
            .add_child(text);
    }

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
            border_color: BorderColor(LegacyColor::WHITE),
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
            border_color: BorderColor(LegacyColor::WHITE),
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
