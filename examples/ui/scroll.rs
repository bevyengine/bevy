//! This example illustrates scrolling in Bevy UI.

use accesskit::{Node as Accessible, Role};
use bevy::{
    a11y::AccessibilityNode,
    color::palettes::css::{BLACK, BLUE, RED},
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
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut commands: Commands,
) {
    for mouse_wheel in mouse_wheel_reader.read() {
        let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);

        if mouse_wheel.unit == MouseScrollUnit::Line {
            delta *= LINE_HEIGHT;
        }

        if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
            std::mem::swap(&mut delta.x, &mut delta.y);
        }

        for pointer_map in hover_map.values() {
            for entity in pointer_map.keys().copied() {
                commands.trigger(Scroll { entity, delta });
            }
        }
    }
}

/// UI scrolling event.
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
struct Scroll {
    entity: Entity,
    /// Scroll delta in logical coordinates.
    delta: Vec2,
}

fn on_scroll_handler(
    mut scroll: On<Scroll>,
    mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode)>,
) {
    let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
        return;
    };

    let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();

    let delta = &mut scroll.delta;
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
        scroll.propagate(false);
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
                            font: font_handle.clone().into(),
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
                                            font: font_handle.clone().into(),
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
                                        |press: On<Pointer<Press>>, mut commands: Commands| {
                                            if press.event().button == PointerButton::Primary {
                                                commands.entity(press.entity).despawn();
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
                    bidirectional_scrolling_list_with_sticky(
                        asset_server.load("fonts/FiraSans-Bold.ttf")
                    ),
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
                    font: font_handle.clone().into(),
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
                    scrollbar_width: 20.,
                    ..default()
                },
                #[cfg(feature = "bevy_ui_debug")]
                UiDebugOptions {
                    enabled: true,
                    outline_border_box: false,
                    outline_padding_box: false,
                    outline_content_box: false,
                    outline_scrollbars: true,
                    line_width: 2.,
                    line_color_override: None,
                    show_hidden: false,
                    show_clipped: true,
                    ignore_border_radius: true,
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
                                font: font_handle.clone().into(),
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
                    font: font_handle.clone().into(),
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
                                    TextFont::from(value.clone()),
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

fn bidirectional_scrolling_list_with_sticky(font_handle: Handle<Font>) -> impl Bundle {
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
                Text::new("Bidirectionally Scrolling List With Sticky Nodes"),
                TextFont {
                    font: font_handle.clone().into(),
                    font_size: FONT_SIZE,
                    ..default()
                },
                Label,
            ),
            (
                Node {
                    display: Display::Grid,
                    align_self: AlignSelf::Stretch,
                    height: percent(50),
                    overflow: Overflow::scroll(), // n.b.
                    grid_template_columns: RepeatedGridTrack::auto(30),
                    ..default()
                },
                Children::spawn(SpawnIter(
                    (0..30)
                        .flat_map(|y| (0..30).map(move |x| (y, x)))
                        .map(move |(y, x)| {
                            let value = font_handle.clone();
                            // Simple sticky nodes at top and left sides of UI node
                            // can be achieved by combining such effects as
                            // IgnoreScroll, ZIndex, BackgroundColor for child UI nodes.
                            let ignore_scroll = BVec2 {
                                x: x == 0,
                                y: y == 0,
                            };
                            let (z_index, background_color, role) = match (x == 0, y == 0) {
                                (true, true) => (2, RED, Role::RowHeader),
                                (true, false) => (1, BLUE, Role::RowHeader),
                                (false, true) => (1, BLUE, Role::ColumnHeader),
                                (false, false) => (0, BLACK, Role::Cell),
                            };
                            (
                                Text(format!("|{},{}|", y, x)),
                                TextFont::from(value.clone()),
                                TextLayout {
                                    linebreak: LineBreak::NoWrap,
                                    ..default()
                                },
                                Label,
                                AccessibilityNode(Accessible::new(role)),
                                IgnoreScroll(ignore_scroll),
                                ZIndex(z_index),
                                BackgroundColor(Color::Srgba(background_color)),
                            )
                        })
                ))
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
                    font: font_handle.clone().into(),
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
                                    TextFont::from(value.clone()),
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
