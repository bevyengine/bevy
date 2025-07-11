//! This example tests scale factor and scrolling

use bevy::color::palettes::css::NAVY;
use bevy::color::palettes::css::RED;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

#[derive(Component, Debug, Eq, PartialEq, Hash)]
enum ValueLabel {
    RenderTargetScaleFactor,
    UiScale,
    CombinedScaleFactor,
    MouseMotionSum,
    DragDeltaSum,
    DragDistance,
    PhysicalNodeSize,
    DragPointerLocation,
    LogicalNodeSize,
    ScrollPosition,
}

#[derive(Resource, Default)]
struct DragValues {
    mouse_motion_sum: Vec2,
    drag_delta_sum: Vec2,
    drag_distance: Vec2,
    drag_start: Vec2,
    drag_pointer_location: Option<Vec2>,
    drag_total: Vec2,
}

#[derive(Component)]
struct DragNode;

#[derive(Component)]
struct ScrollableNode;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<DragValues>()
        .add_systems(Startup, setup)
        .add_systems(Update, update)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    commands.insert_resource(UiScale(0.5));
    commands
        .spawn((
            Pickable::IGNORE,
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                flex_direction: FlexDirection::Column,
                ..Default::default()
            },
        ))
        .with_children(|commands| {
            for w in [
                ValueLabel::RenderTargetScaleFactor,
                ValueLabel::UiScale,
                ValueLabel::CombinedScaleFactor,
                ValueLabel::MouseMotionSum,
                ValueLabel::DragDeltaSum,
                ValueLabel::DragDistance,
                ValueLabel::PhysicalNodeSize,
                ValueLabel::DragPointerLocation,
                ValueLabel::LogicalNodeSize,
                ValueLabel::ScrollPosition,
            ] {
                commands.spawn((
                    Text::new(format!("{w:?}: ")),
                    children![(TextSpan::default(), w)],
                ));
            }
        });

    commands
        .spawn((
            Pickable {
                should_block_lower: false,
                is_hoverable: true,
            },
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                ..Default::default()
            },
            GlobalZIndex(1),
            children![(
                Node {
                    position_type: PositionType::Absolute,
                    border: UiRect::all(Val::Px(2.)),
                    ..default()
                },
                Pickable::IGNORE,
                BorderColor::all(RED.into()),
                DragNode,
            )],
        ))
        .observe(|drag: On<Pointer<Drag>>, mut values: ResMut<DragValues>| {
            values.drag_distance = drag.distance;
            values.drag_delta_sum += drag.delta;
            values.drag_total += drag.delta;
            values.drag_total.x = values.drag_total.x.min(0.);
            values.drag_total.y = values.drag_total.x.min(0.);
            values.drag_pointer_location = Some(drag.pointer_location.position);
        })
        .observe(
            |drag: On<Pointer<DragStart>>, mut values: ResMut<DragValues>| {
                values.drag_distance = Vec2::ZERO;
                values.drag_delta_sum = Vec2::ZERO;
                values.mouse_motion_sum = Vec2::ZERO;
                values.drag_start = drag.pointer_location.position;
                values.drag_pointer_location = Some(drag.pointer_location.position);
            },
        );

    commands
        .spawn((
            Node {
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                overflow: Overflow::scroll(),
                ..Default::default()
            },
            ScrollPosition(Vec2::ZERO),
            ScrollableNode,
            GlobalZIndex(-1),
        ))
        .with_children(|commands| {
            for x in 0..30 {
                commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        for y in 0..30 {
                            let color = if (x + y) % 2 == 1 {
                                NAVY.into()
                            } else {
                                Color::BLACK
                            };
                            commands.spawn((
                                Node {
                                    width: Val::Px(100.),
                                    height: Val::Px(100.),
                                    min_height: Val::Px(100.),
                                    ..Default::default()
                                },
                                BackgroundColor(color),
                            ));
                        }
                    });
            }
        });
}

fn update(
    mut mouse_motions: EventReader<MouseMotion>,
    ui_scale: Res<UiScale>,
    mut values: ResMut<DragValues>,
    mut watched_values_query: Query<(&ValueLabel, &mut TextSpan)>,
    mut drag_node_query: Query<(&ComputedNode, &mut Node, &ComputedNodeTarget), With<DragNode>>,
    mut scroll_position_query: Query<&mut ScrollPosition, With<ScrollableNode>>,
) {
    let Ok((computed_node, mut node, computed_target)) = drag_node_query.single_mut() else {
        return;
    };

    for motion in mouse_motions.read() {
        values.mouse_motion_sum += motion.delta;
    }

    let current_scroll_position = scroll_position_query.single_mut().ok().map(|mut p| {
        let out = Vec2::new(p.x, p.y);
        p.x = -values.drag_total.x;
        p.y = -values.drag_total.y;

        out
    });

    for (value, mut text_span) in watched_values_query.iter_mut() {
        text_span.0 = match value {
            ValueLabel::RenderTargetScaleFactor => {
                (computed_target.scale_factor() / ui_scale.0).to_string()
            }
            ValueLabel::UiScale => ui_scale.0.to_string(),
            ValueLabel::CombinedScaleFactor => computed_target.scale_factor().to_string(),
            ValueLabel::MouseMotionSum => values.mouse_motion_sum.to_string(),
            ValueLabel::DragDeltaSum => values.drag_delta_sum.to_string(),
            ValueLabel::DragDistance => values.drag_distance.to_string(),
            ValueLabel::PhysicalNodeSize => computed_node.size().to_string(),
            ValueLabel::LogicalNodeSize => {
                (computed_node.size() / computed_target.scale_factor()).to_string()
            }
            ValueLabel::DragPointerLocation => format!("{:?}", values.drag_pointer_location),
            ValueLabel::ScrollPosition => format!("{:?}", current_scroll_position),
        }
    }

    let logical_drag_distance = values.drag_distance / ui_scale.0;
    let logical_drag_start = values.drag_start / ui_scale.0;
    node.width = Val::Px(logical_drag_distance.x.abs());
    node.height = Val::Px(logical_drag_distance.y.abs());
    if logical_drag_distance.x < 0. {
        node.left = Val::Px(logical_drag_start.x + logical_drag_distance.x);
    } else {
        node.left = Val::Px(logical_drag_start.x);
    }

    if logical_drag_distance.y < 0. {
        node.top = Val::Px(logical_drag_start.y + logical_drag_distance.y);
    } else {
        node.top = Val::Px(logical_drag_start.y);
    }
}
