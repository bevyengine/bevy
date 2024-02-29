//! This example demonstrates animation blending with animation graphs.

use bevy::{math::vec2, prelude::*};
use ron::de;

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

static NODE_RECTS: [NodeRect; 5] = [
    NodeRect {
        left: 300f32,
        bottom: 375.0010000000002f32,
        width: 390.541f32,
        height: 193.629f32,
    },
    NodeRect {
        left: 300f32,
        bottom: 648.6310000000001f32,
        width: 390.541f32,
        height: 193.629f32,
    },
    NodeRect {
        left: 1404.32f32,
        bottom: 648.6310000000001f32,
        width: 390.541f32,
        height: 193.629f32,
    },
    NodeRect {
        left: 852.161f32,
        bottom: 511.81100000000015f32,
        width: 390.541f32,
        height: 193.629f32,
    },
    NodeRect {
        left: 300f32,
        bottom: 922.2610000000002f32,
        width: 390.541f32,
        height: 193.629f32,
    },
];

static HORIZONTAL_LINES: [Line; 6] = [
    Line {
        left: 690.541,
        bottom: 471.82000000000016,
        length: 80.78399999999999,
    },
    Line {
        left: 690.541,
        bottom: 745.44,
        length: 80.78399999999999,
    },
    Line {
        left: 690.541,
        bottom: 1019.0699999999999,
        length: 632.9689999999999,
    },
    Line {
        left: 771.351,
        bottom: 608.6300000000001,
        length: 80.80999999999995,
    },
    Line {
        left: 1242.7,
        bottom: 608.6300000000001,
        length: 80.80999999999995,
    },
    Line {
        left: 1323.51,
        bottom: 745.44,
        length: 80.80999999999995,
    },
];
static VERTICAL_LINES: [Line; 2] = [
    Line {
        left: 771.325,
        bottom: 471.82000000000016,
        length: 273.6199999999999,
    },
    Line {
        left: 1323.51,
        bottom: 608.6300000000001,
        length: 410.4399999999998,
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

    for node_rect in &NODE_RECTS {
        commands.spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(node_rect.bottom),
                left: Val::Px(node_rect.left),
                height: Val::Px(node_rect.height),
                width: Val::Px(node_rect.width),
                border: UiRect::all(Val::Px(1.0)),
                ..default()
            },
            border_color: BorderColor(LegacyColor::WHITE),
            ..default()
        });
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
