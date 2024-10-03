//! This example illustrates scrolling in Bevy UI.

use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::focus::HoverMap,
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
    commands.spawn((Camera2dBundle::default(), IsDefaultUiCamera));

    // root node
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceBetween,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .insert(Pickable::IGNORE)
        .with_children(|parent| {
            // horizontal scroll example
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.),
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // header
                    parent.spawn((
                        TextNEW::new("Horizontally Scrolling list (Ctrl + Mousewheel)"),
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: FONT_SIZE,
                            ..default()
                        },
                        Label,
                    ));

                    // horizontal scroll container
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(80.),
                                margin: UiRect::all(Val::Px(10.)),
                                flex_direction: FlexDirection::Row,
                                overflow: Overflow::scroll_x(), // n.b.
                                ..default()
                            },
                            background_color: Color::srgb(0.10, 0.10, 0.10).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            for i in 0..100 {
                                parent.spawn((TextNEW(format!("Item {i}")),
                                        TextStyle {
                                            font: asset_server
                                                .load("fonts/FiraSans-Bold.ttf"),
                                            ..default()
                                        },
                                    Label,
                                    AccessibilityNode(NodeBuilder::new(Role::ListItem)),
                                ))
                                .insert(Style {
                                    min_width: Val::Px(200.),
                                    align_content: AlignContent::Center,
                                    ..default()
                                })
                                .insert(Pickable {
                                    should_block_lower: false,
                                    ..default()
                                })
                                .observe(|
                                    trigger: Trigger<Pointer<Down>>,
                                    mut commands: Commands
                                | {
                                    if trigger.event().button == PointerButton::Primary {
                                        commands.entity(trigger.entity()).despawn_recursive();
                                    }
                                });
                            }
                        });
                });

            // container for all other examples
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // vertical scroll example
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                width: Val::Px(200.),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|parent| {
                            // Title
                            parent.spawn((
                                TextNEW::new("Vertically Scrolling List"),
                                TextStyle {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: FONT_SIZE,
                                    ..default()
                                },
                                Label,
                            ));
                            // Scrolling list
                            parent
                                .spawn(NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Column,
                                        align_self: AlignSelf::Stretch,
                                        height: Val::Percent(50.),
                                        overflow: Overflow::scroll_y(), // n.b.
                                        ..default()
                                    },
                                    background_color: Color::srgb(0.10, 0.10, 0.10).into(),
                                    ..default()
                                })
                                .with_children(|parent| {
                                    // List items
                                    for i in 0..25 {
                                        parent
                                            .spawn(NodeBundle {
                                                style: Style {
                                                    min_height: Val::Px(LINE_HEIGHT),
                                                    max_height: Val::Px(LINE_HEIGHT),
                                                    ..default()
                                                },
                                                ..default()
                                            })
                                            .insert(Pickable {
                                                should_block_lower: false,
                                                ..default()
                                            })
                                            .with_children(|parent| {
                                                parent
                                                    .spawn((
                                                        TextNEW(format!("Item {i}")),
                                                        TextStyle {
                                                            font: asset_server
                                                                .load("fonts/FiraSans-Bold.ttf"),
                                                            ..default()
                                                        },
                                                        Label,
                                                        AccessibilityNode(NodeBuilder::new(
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
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                width: Val::Px(200.),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|parent| {
                            // Title
                            parent.spawn((
                                TextNEW::new("Bidirectionally Scrolling List"),
                                TextStyle {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: FONT_SIZE,
                                    ..default()
                                },
                                Label,
                            ));
                            // Scrolling list
                            parent
                                .spawn(NodeBundle {
                                    style: Style {
                                        flex_direction: FlexDirection::Column,
                                        align_self: AlignSelf::Stretch,
                                        height: Val::Percent(50.),
                                        overflow: Overflow::scroll(), // n.b.
                                        ..default()
                                    },
                                    background_color: Color::srgb(0.10, 0.10, 0.10).into(),
                                    ..default()
                                })
                                .with_children(|parent| {
                                    // Rows in each column
                                    for oi in 0..10 {
                                        parent
                                            .spawn(NodeBundle {
                                                style: Style {
                                                    flex_direction: FlexDirection::Row,
                                                    ..default()
                                                },
                                                ..default()
                                            })
                                            .insert(Pickable::IGNORE)
                                            .with_children(|parent| {
                                                // Elements in each row
                                                for i in 0..25 {
                                                    parent
                                                        .spawn((
                                                            TextNEW(format!(
                                                                "Item {}",
                                                                (oi * 25) + i
                                                            )),
                                                            TextStyle {
                                                                font: asset_server.load(
                                                                    "fonts/FiraSans-Bold.ttf",
                                                                ),
                                                                ..default()
                                                            },
                                                            Label,
                                                            AccessibilityNode(NodeBuilder::new(
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
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                align_items: AlignItems::Center,
                                width: Val::Px(200.),
                                ..default()
                            },
                            ..default()
                        })
                        .with_children(|parent| {
                            // Title
                            parent.spawn((
                                TextNEW::new("Nested Scrolling Lists"),
                                TextStyle {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: FONT_SIZE,
                                    ..default()
                                },
                                Label,
                            ));
                            // Outer, horizontal scrolling container
                            parent
                                .spawn(NodeBundle {
                                    style: Style {
                                        column_gap: Val::Px(20.),
                                        flex_direction: FlexDirection::Row,
                                        align_self: AlignSelf::Stretch,
                                        height: Val::Percent(50.),
                                        overflow: Overflow::scroll_x(), // n.b.
                                        ..default()
                                    },
                                    background_color: Color::srgb(0.10, 0.10, 0.10).into(),
                                    ..default()
                                })
                                .with_children(|parent| {
                                    // Inner, scrolling columns
                                    for oi in 0..30 {
                                        parent
                                            .spawn(NodeBundle {
                                                style: Style {
                                                    flex_direction: FlexDirection::Column,
                                                    align_self: AlignSelf::Stretch,
                                                    overflow: Overflow::scroll_y(),
                                                    ..default()
                                                },
                                                background_color: Color::srgb(0.05, 0.05, 0.05)
                                                    .into(),
                                                ..default()
                                            })
                                            .insert(Pickable {
                                                should_block_lower: false,
                                                ..default()
                                            })
                                            .with_children(|parent| {
                                                for i in 0..25 {
                                                    parent
                                                        .spawn((
                                                            TextNEW(format!(
                                                                "Item {}",
                                                                (oi * 25) + i
                                                            )),
                                                            TextStyle {
                                                                font: asset_server.load(
                                                                    "fonts/FiraSans-Bold.ttf",
                                                                ),
                                                                ..default()
                                                            },
                                                            Label,
                                                            AccessibilityNode(NodeBuilder::new(
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
