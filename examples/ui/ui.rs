//! This example illustrates the various features of Bevy UI.

use bevy::{
    a11y::{
        accesskit::{NodeBuilder, Role},
        AccessibilityNode,
    },
    color::palettes::basic::LIME,
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::focus::HoverMap,
    prelude::*,
    winit::WinitSettings,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, update_scroll_position);

    #[cfg(feature = "bevy_dev_tools")]
    {
        app.add_plugins(bevy::dev_tools::ui_debug_overlay::DebugUiPlugin)
            .add_systems(Update, toggle_overlay);
    }

    app.run();
}

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
                ..default()
            },
            ..default()
        })
        .insert(Pickable::IGNORE)
        .with_children(|parent| {
            // left vertical fill (border)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(200.),
                        border: UiRect::all(Val::Px(2.)),
                        ..default()
                    },
                    background_color: Color::srgb(0.65, 0.65, 0.65).into(),
                    ..default()
                })
                .with_children(|parent| {
                    // left vertical fill (content)
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Percent(100.),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(5.)),
                                row_gap: Val::Px(5.),
                                ..default()
                            },
                            background_color: Color::srgb(0.15, 0.15, 0.15).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            // text
                            parent.spawn((
                                TextNEW::new("Text Example"),
                                TextStyle {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: 25.0,
                                    ..default()
                                },
                                // Because this is a distinct label widget and
                                // not button/list item text, this is necessary
                                // for accessibility to treat the text accordingly.
                                Label,
                            ));

                            #[cfg(feature = "bevy_dev_tools")]
                            // Debug overlay text
                            parent.spawn((
                                TextNEW::new("Press Space to enable debug outlines."),
                                TextStyle {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    ..default()
                                },
                                Label,
                            ));

                            #[cfg(not(feature = "bevy_dev_tools"))]
                            parent.spawn((
                                TextNEW::new("Try enabling feature \"bevy_dev_tools\"."),
                                TextStyle {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    ..default()
                                },
                                Label,
                            ));
                        });
                });
            // right vertical fill
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
                        TextNEW::new("Scrolling list"),
                        TextStyle {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 21.,
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
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            background_color: Color::srgb(0.10, 0.10, 0.10).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            // List items
                            for i in 0..25 {
                                parent
                                    .spawn((
                                        TextNEW(format!("Item {i}")),
                                        TextStyle {
                                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                            ..default()
                                        },
                                        Label,
                                        AccessibilityNode(NodeBuilder::new(Role::ListItem)),
                                    ))
                                    .insert(Pickable {
                                        should_block_lower: false,
                                        ..default()
                                    });
                            }
                        });
                });

            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(200.0),
                        height: Val::Px(200.0),
                        position_type: PositionType::Absolute,
                        left: Val::Px(210.),
                        bottom: Val::Px(10.),
                        border: UiRect::all(Val::Px(20.)),
                        ..default()
                    },
                    border_color: LIME.into(),
                    background_color: Color::srgb(0.4, 0.4, 1.).into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(NodeBundle {
                        style: Style {
                            width: Val::Percent(100.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: Color::srgb(0.8, 0.8, 1.).into(),
                        ..default()
                    });
                });
            // render order test: reddest in the back, whitest in the front (flex center)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    ..default()
                })
                .insert(Pickable::IGNORE)
                .with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.0),
                                height: Val::Px(100.0),
                                ..default()
                            },
                            background_color: Color::srgb(1.0, 0.0, 0.).into(),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn(NodeBundle {
                                style: Style {
                                    // Take the size of the parent node.
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(20.),
                                    bottom: Val::Px(20.),
                                    ..default()
                                },
                                background_color: Color::srgb(1.0, 0.3, 0.3).into(),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(40.),
                                    bottom: Val::Px(40.),
                                    ..default()
                                },
                                background_color: Color::srgb(1.0, 0.5, 0.5).into(),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(60.),
                                    bottom: Val::Px(60.),
                                    ..default()
                                },
                                background_color: Color::srgb(1.0, 0.7, 0.7).into(),
                                ..default()
                            });
                            // alpha test
                            parent.spawn(NodeBundle {
                                style: Style {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(80.),
                                    bottom: Val::Px(80.),
                                    ..default()
                                },
                                background_color: Color::srgba(1.0, 0.9, 0.9, 0.4).into(),
                                ..default()
                            });
                        });
                });
            // bevy logo (flex center)
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(100.0),
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexStart,
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // bevy logo (image)
                    // A `NodeBundle` is used to display the logo the image as an `ImageBundle` can't automatically
                    // size itself with a child node present.
                    parent
                        .spawn((
                            NodeBundle {
                                style: Style {
                                    width: Val::Px(500.0),
                                    height: Val::Px(125.0),
                                    margin: UiRect::top(Val::VMin(5.)),
                                    ..default()
                                },
                                ..default()
                            },
                            UiImage::new(asset_server.load("branding/bevy_logo_dark_big.png")),
                        ))
                        .with_children(|parent| {
                            // alt text
                            // This UI node takes up no space in the layout and the `Text` component is used by the accessibility module
                            // and is not rendered.
                            parent.spawn((
                                NodeBundle {
                                    style: Style {
                                        display: Display::None,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                TextNEW::new("Bevy logo"),
                            ));
                        });
                });
        });
}

#[cfg(feature = "bevy_dev_tools")]
// The system that will enable/disable the debug outlines around the nodes
fn toggle_overlay(
    input: Res<ButtonInput<KeyCode>>,
    mut options: ResMut<bevy::dev_tools::ui_debug_overlay::UiDebugOptions>,
) {
    info_once!("The debug outlines are enabled, press Space to turn them on/off");
    if input.just_pressed(KeyCode::Space) {
        // The toggle method will enable the debug_overlay if disabled and disable if enabled
        options.toggle();
    }
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
            MouseScrollUnit::Line => (mouse_wheel_event.x * 20., mouse_wheel_event.y * 20.),
            MouseScrollUnit::Pixel => (mouse_wheel_event.x, mouse_wheel_event.y),
        };

        if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight)
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
