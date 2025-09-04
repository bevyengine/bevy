//! This example illustrates scrolling in Bevy UI.

use accesskit::{Node as Accessible, Role};
use bevy::{
    a11y::AccessibilityNode,
    ecs::spawn::SpawnIter,
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::HoverMap,
    prelude::*,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, send_scroll_events)
        .add_observer(on_scroll_handler);

    app.run();
}

const LINE_HEIGHT: f32 = 21.;

/// Injects scroll events into the UI hierarchy.
fn send_scroll_events(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for event in mouse_wheel_events.read() {
        let mut delta = -Vec2::new(event.x, event.y);

        if event.unit == MouseScrollUnit::Line {
            delta *= LINE_HEIGHT;
        }

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys() {
                commands.trigger_targets(Scroll { delta }, *entity);
            }
        }
    }
}

/// UI scrolling event.
#[derive(EntityEvent, Debug)]
#[entity_event(auto_propagate, traversal = &'static ChildOf)]
struct Scroll {
    /// Scroll delta in logical coordinates.
    delta: Vec2,
}

fn on_scroll_handler(
    mut event: On<Scroll>,
    mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let target = event.entity();
    let delta = &mut event.delta;

    let Ok((mut scroll_position, node, computed)) = query.get_mut(target) else {
        return;
    };

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.x > 0. {
            scroll_position.x >= max_offset.x
        } else {
            scroll_position.x <= 0.
        };

        if !max {
            scroll_position.x += delta.x;
            // Consume the X portion of the scroll delta.
            delta.x = 0.;
        }
    }

    if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
        // Is this node already scrolled all the way in the direction of the scroll?
        let max = if delta.y > 0. {
            scroll_position.y >= max_offset.y
        } else {
            scroll_position.y <= 0.
        };

        if !max {
            scroll_position.y += delta.y;
            // Consume the Y portion of the scroll delta.
            delta.y = 0.;
        }
    }

    // Stop propagating when the delta is fully consumed.
    if *delta == Vec2::ZERO {
        event.propagate(false);
    }
}

const FONT_SIZE: f32 = 20.;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn((Camera2d, IsDefaultUiCamera));

    // Font
    let font_handle = asset_server.load("fonts/FiraSans-Bold.ttf");

    // root node
    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            justify_content: JustifyContent::SpaceBetween,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .with_children(|parent| {
            // horizontal scroll example
            parent
                .spawn(Node {
                    width: percent(100),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|parent| {
                    // header
                    parent.spawn((
                        Text::new("Horizontally Scrolling list (Ctrl + MouseWheel)"),
                        TextFont {
                            font: font_handle.clone(),
                            font_size: FONT_SIZE,
                            ..default()
                        },
                        Label,
                    ));

                    // horizontal scroll container
                    parent
                        .spawn((
                            Node {
                                width: percent(80),
                                margin: UiRect::all(px(10)),
                                flex_direction: FlexDirection::Row,
                                overflow: Overflow::scroll_x(), // n.b.
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                        ))
                        .with_children(|parent| {
                            for i in 0..100 {
                                parent
                                    .spawn((
                                        Text(format!("Item {i}")),
                                        TextFont {
                                            font: font_handle.clone(),
                                            ..default()
                                        },
                                        Label,
                                        AccessibilityNode(Accessible::new(Role::ListItem)),
                                        Node {
                                            min_width: px(200),
                                            align_content: AlignContent::Center,
                                            ..default()
                                        },
                                    ))
                                    .observe(
                                        |event: On<Pointer<Press>>, mut commands: Commands| {
                                            if event.event().button == PointerButton::Primary {
                                                commands.entity(event.entity()).despawn();
                                            }
                                        },
                                    );
                            }
                        });
                });

            // container for all other examples
            parent.spawn((
                Node {
                    width: percent(100),
                    height: percent(100),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                children![
                    vertically_scrolling_list(asset_server.load("fonts/FiraSans-Bold.ttf")),
                    bidirectional_scrolling_list(asset_server.load("fonts/FiraSans-Bold.ttf")),
                    nested_scrolling_list(asset_server.load("fonts/FiraSans-Bold.ttf")),
                ],
            ));
        });
}

fn vertically_scrolling_list(font_handle: Handle<Font>) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            width: px(200),
            ..default()
        },
        children![
            (
                // Title
                Text::new("Vertically Scrolling List"),
                TextFont {
                    font: font_handle.clone(),
                    font_size: FONT_SIZE,
                    ..default()
                },
                Label,
            ),
            (
                // Scrolling list
                Node {
                    flex_direction: FlexDirection::Column,
                    align_self: AlignSelf::Stretch,
                    height: percent(50),
                    overflow: Overflow::scroll_y(), // n.b.
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                Children::spawn(SpawnIter((0..25).map(move |i| {
                    (
                        Node {
                            min_height: px(LINE_HEIGHT),
                            max_height: px(LINE_HEIGHT),
                            ..default()
                        },
                        children![(
                            Text(format!("Item {i}")),
                            TextFont {
                                font: font_handle.clone(),
                                ..default()
                            },
                            Label,
                            AccessibilityNode(Accessible::new(Role::ListItem)),
                        )],
                    )
                })))
            ),
        ],
    )
}

fn bidirectional_scrolling_list(font_handle: Handle<Font>) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            width: px(200),
            ..default()
        },
        children![
            (
                Text::new("Bidirectionally Scrolling List"),
                TextFont {
                    font: font_handle.clone(),
                    font_size: FONT_SIZE,
                    ..default()
                },
                Label,
            ),
            (
                Node {
                    flex_direction: FlexDirection::Column,
                    align_self: AlignSelf::Stretch,
                    height: percent(50),
                    overflow: Overflow::scroll(), // n.b.
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                Children::spawn(SpawnIter((0..25).map(move |oi| {
                    (
                        Node {
                            flex_direction: FlexDirection::Row,
                            ..default()
                        },
                        Children::spawn(SpawnIter((0..10).map({
                            let value = font_handle.clone();
                            move |i| {
                                (
                                    Text(format!("Item {}", (oi * 10) + i)),
                                    TextFont {
                                        font: value.clone(),
                                        ..default()
                                    },
                                    Label,
                                    AccessibilityNode(Accessible::new(Role::ListItem)),
                                )
                            }
                        }))),
                    )
                })))
            )
        ],
    )
}

fn nested_scrolling_list(font_handle: Handle<Font>) -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            width: px(200),
            ..default()
        },
        children![
            (
                // Title
                Text::new("Nested Scrolling Lists"),
                TextFont {
                    font: font_handle.clone(),
                    font_size: FONT_SIZE,
                    ..default()
                },
                Label,
            ),
            (
                // Outer, bi-directional scrolling container
                Node {
                    column_gap: px(20),
                    flex_direction: FlexDirection::Row,
                    align_self: AlignSelf::Stretch,
                    height: percent(50),
                    overflow: Overflow::scroll(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                // Inner, scrolling columns
                Children::spawn(SpawnIter((0..5).map(move |oi| {
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_self: AlignSelf::Stretch,
                            height: percent(200. / 5. * (oi as f32 + 1.)),
                            overflow: Overflow::scroll_y(),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                        Children::spawn(SpawnIter((0..20).map({
                            let value = font_handle.clone();
                            move |i| {
                                (
                                    Text(format!("Item {}", (oi * 20) + i)),
                                    TextFont {
                                        font: value.clone(),
                                        ..default()
                                    },
                                    Label,
                                    AccessibilityNode(Accessible::new(Role::ListItem)),
                                )
                            }
                        }))),
                    )
                })))
            )
        ],
    )
}
