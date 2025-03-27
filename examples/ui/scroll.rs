//! This example illustrates scrolling in Bevy UI.

use accesskit::{Node as Accessible, Role};
use bevy::{
    a11y::AccessibilityNode,
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::HoverMap,
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, update_scroll_position);

    app.run();
}

const FONT_SIZE: f32 = 20.;
const LINE_HEIGHT: f32 = 21.;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn((Camera2d, IsDefaultUiCamera));

    // root node
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .insert(Pickable::IGNORE)
        .with_children(|parent| {
            // horizontal scroll example
            parent
                .spawn(Node {
                    width: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|parent| {
                    // header
                    parent.spawn((
                        Text::new("Horizontally Scrolling list (Ctrl + MouseWheel)"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: FONT_SIZE,
                            ..default()
                        },
                        Label,
                    ));

                    // horizontal scroll container
                    parent
                        .spawn((
                            Node {
                                width: Val::Percent(80.),
                                margin: UiRect::all(Val::Px(10.)),
                                flex_direction: FlexDirection::Row,
                                overflow: Overflow::scroll_x(), // n.b.
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                        ))
                        .with_children(|parent| {
                            for i in 0..100 {
                                parent.spawn((Text(format!("Item {i}")),
                                        TextFont {
                                            font: asset_server
                                                .load("fonts/FiraSans-Bold.ttf"),
                                            ..default()
                                        },
                                    Label,
                                    AccessibilityNode(Accessible::new(Role::ListItem)),
                                ))
                                .insert(Node {
                                    min_width: Val::Px(200.),
                                    align_content: AlignContent::Center,
                                    ..default()
                                })
                                .insert(Pickable {
                                    should_block_lower: false,
                                    ..default()
                                })
                                .observe(|
                                    trigger: Trigger<Pointer<Pressed>>,
                                    mut commands: Commands
                                | {
                                    if trigger.event().button == PointerButton::Primary {
                                        commands.entity(trigger.target()).despawn();
                                    }
                                });
                            }
                        });
                });

            // container for all other examples
            parent
                .spawn(Node {
                    width: Val::Percent(100.),
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                })
                .with_children(|parent| {
                    // vertical scroll example
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            width: Val::Px(200.),
                            ..default()
                        })
                        .with_children(|parent| {
                            // Title
                            parent.spawn((
                                Text::new("Vertically Scrolling List"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: FONT_SIZE,
                                    ..default()
                                },
                                Label,
                            ));
                            // Scrolling list
                            parent
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        align_self: AlignSelf::Stretch,
                                        height: Val::Percent(50.),
                                        overflow: Overflow::scroll_y(), // n.b.
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                                ))
                                .with_children(|parent| {
                                    // List items
                                    for i in 0..25 {
                                        parent
                                            .spawn(Node {
                                                min_height: Val::Px(LINE_HEIGHT),
                                                max_height: Val::Px(LINE_HEIGHT),
                                                ..default()
                                            })
                                            .insert(Pickable {
                                                should_block_lower: false,
                                                ..default()
                                            })
                                            .with_children(|parent| {
                                                parent
                                                    .spawn((
                                                        Text(format!("Item {i}")),
                                                        TextFont {
                                                            font: asset_server
                                                                .load("fonts/FiraSans-Bold.ttf"),
                                                            ..default()
                                                        },
                                                        Label,
                                                        AccessibilityNode(Accessible::new(
                                                            Role::ListItem,
                                                        )),
                                                    ))
                                                    .insert(Pickable {
                                                        should_block_lower: false,
                                                        ..default()
                                                    });
                                            });
                                    }
                                });
                        });

                    // Bidirectional scroll example
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            width: Val::Px(200.),
                            ..default()
                        })
                        .with_children(|parent| {
                            // Title
                            parent.spawn((
                                Text::new("Bidirectionally Scrolling List"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: FONT_SIZE,
                                    ..default()
                                },
                                Label,
                            ));
                            // Scrolling list
                            parent
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        align_self: AlignSelf::Stretch,
                                        height: Val::Percent(50.),
                                        overflow: Overflow::scroll(), // n.b.
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                                ))
                                .with_children(|parent| {
                                    // Rows in each column
                                    for oi in 0..10 {
                                        parent
                                            .spawn(Node {
                                                flex_direction: FlexDirection::Row,
                                                ..default()
                                            })
                                            .insert(Pickable::IGNORE)
                                            .with_children(|parent| {
                                                // Elements in each row
                                                for i in 0..25 {
                                                    parent
                                                        .spawn((
                                                            Text(format!("Item {}", (oi * 25) + i)),
                                                            TextFont {
                                                                font: asset_server.load(
                                                                    "fonts/FiraSans-Bold.ttf",
                                                                ),
                                                                ..default()
                                                            },
                                                            Label,
                                                            AccessibilityNode(Accessible::new(
                                                                Role::ListItem,
                                                            )),
                                                        ))
                                                        .insert(Pickable {
                                                            should_block_lower: false,
                                                            ..default()
                                                        });
                                                }
                                            });
                                    }
                                });
                        });

                    // Nested scrolls example
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            width: Val::Px(200.),
                            ..default()
                        })
                        .with_children(|parent| {
                            // Title
                            parent.spawn((
                                Text::new("Nested Scrolling Lists"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: FONT_SIZE,
                                    ..default()
                                },
                                Label,
                            ));
                            // Outer, horizontal scrolling container
                            parent
                                .spawn((
                                    Node {
                                        column_gap: Val::Px(20.),
                                        flex_direction: FlexDirection::Row,
                                        align_self: AlignSelf::Stretch,
                                        height: Val::Percent(50.),
                                        overflow: Overflow::scroll_x(), // n.b.
                                        ..default()
                                    },
                                    BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                                ))
                                .with_children(|parent| {
                                    // Inner, scrolling columns
                                    for oi in 0..30 {
                                        parent
                                            .spawn((
                                                Node {
                                                    flex_direction: FlexDirection::Column,
                                                    align_self: AlignSelf::Stretch,
                                                    overflow: Overflow::scroll_y(),
                                                    ..default()
                                                },
                                                BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                                            ))
                                            .insert(Pickable {
                                                should_block_lower: false,
                                                ..default()
                                            })
                                            .with_children(|parent| {
                                                for i in 0..25 {
                                                    parent
                                                        .spawn((
                                                            Text(format!("Item {}", (oi * 25) + i)),
                                                            TextFont {
                                                                font: asset_server.load(
                                                                    "fonts/FiraSans-Bold.ttf",
                                                                ),
                                                                ..default()
                                                            },
                                                            Label,
                                                            AccessibilityNode(Accessible::new(
                                                                Role::ListItem,
                                                            )),
                                                        ))
                                                        .insert(Pickable {
                                                            should_block_lower: false,
                                                            ..default()
                                                        });
                                                }
                                            });
                                    }
                                });
                        });
                });
        });
}

/// Updates the scroll position of scrollable nodes in response to mouse input
pub fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scrolled_node_query: Query<&mut ScrollPosition>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    for mouse_wheel_event in mouse_wheel_events.read() {
        let (mut dx, mut dy) = match mouse_wheel_event.unit {
            MouseScrollUnit::Line => (
                mouse_wheel_event.x * LINE_HEIGHT,
                mouse_wheel_event.y * LINE_HEIGHT,
            ),
            MouseScrollUnit::Pixel => (mouse_wheel_event.x, mouse_wheel_event.y),
        };

        if keyboard_input.pressed(KeyCode::ControlLeft)
            || keyboard_input.pressed(KeyCode::ControlRight)
        {
            std::mem::swap(&mut dx, &mut dy);
        }

        for (_pointer, pointer_map) in hover_map.iter() {
            for (entity, _hit) in pointer_map.iter() {
                if let Ok(mut scroll_position) = scrolled_node_query.get_mut(*entity) {
                    scroll_position.offset_x -= dx;
                    scroll_position.offset_y -= dy;
                }
            }
        }
    }
}
