//! This example illustrates the various features of Bevy UI.

use std::f32::consts::PI;

use accesskit::{Node as Accessible, Role};
use bevy::{
    a11y::AccessibilityNode,
    color::palettes::{
        basic::LIME,
        css::{DARK_GRAY, NAVY},
    },
    core_widgets::CoreScrollbar,
    input::mouse::{MouseScrollUnit, MouseWheel},
    picking::hover::HoverMap,
    prelude::*,
    ui::widget::NodeImageMode,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_scroll_position);

    #[cfg(feature = "bevy_ui_debug")]
    app.add_systems(Update, toggle_debug_overlay);

    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Camera
    commands.spawn((Camera2d, IsDefaultUiCamera, BoxShadowSamples(6)));

    // root node
    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::SpaceBetween,
            ..default()
        })
        .insert(Pickable::IGNORE)
        .with_children(|parent| {
            // left vertical fill (border)
            parent
                .spawn((
                    Node {
                        width: Val::Px(200.),
                        border: UiRect::all(Val::Px(2.)),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.65, 0.65, 0.65)),
                ))
                .with_children(|parent| {
                    // left vertical fill (content)
                    parent
                        .spawn((
                            Node {
                                width: Val::Percent(100.),
                                flex_direction: FlexDirection::Column,
                                padding: UiRect::all(Val::Px(5.)),
                                row_gap: Val::Px(5.),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                            Visibility::Visible,
                        ))
                        .with_children(|parent| {
                            // text
                            parent.spawn((
                                Text::new("Text Example"),
                                TextFont {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    font_size: 25.0,
                                    ..default()
                                },
                                // Because this is a distinct label widget and
                                // not button/list item text, this is necessary
                                // for accessibility to treat the text accordingly.
                                Label,
                            ));

                            #[cfg(feature = "bevy_ui_debug")]
                            {
                                // Debug overlay text
                                parent.spawn((
                                    Text::new("Press Space to toggle debug outlines."),
                                    TextFont {
                                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                        ..default()
                                    },
                                    Label,
                                ));

                                parent.spawn((
                                    Text::new("V: toggle UI root's visibility"),
                                    TextFont {
                                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                        font_size: 12.,
                                        ..default()
                                    },
                                    Label,
                                ));

                                parent.spawn((
                                    Text::new("S: toggle outlines for hidden nodes"),
                                    TextFont {
                                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                        font_size: 12.,
                                        ..default()
                                    },
                                    Label,
                                ));
                                parent.spawn((
                                    Text::new("C: toggle outlines for clipped nodes"),
                                    TextFont {
                                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                        font_size: 12.,
                                        ..default()
                                    },
                                    Label,
                                ));
                            }
                            #[cfg(not(feature = "bevy_ui_debug"))]
                            parent.spawn((
                                Text::new("Try enabling feature \"bevy_ui_debug\"."),
                                TextFont {
                                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                    ..default()
                                },
                                Label,
                            ));
                        });
                });
            // right vertical fill
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
                        Text::new("Scrolling list"),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                            font_size: 21.,
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
                                overflow: Overflow::scroll_y(),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(0.10, 0.10, 0.10)),
                        ))
                        .with_children(|parent| {
                            parent
                                .spawn((
                                    Node {
                                        flex_direction: FlexDirection::Column,
                                        ..Default::default()
                                    },
                                    BackgroundGradient::from(LinearGradient::to_bottom(vec![
                                        ColorStop::auto(NAVY),
                                        ColorStop::auto(Color::BLACK),
                                    ])),
                                    Pickable {
                                        should_block_lower: false,
                                        ..Default::default()
                                    },
                                ))
                                .with_children(|parent| {
                                    // List items
                                    for i in 0..25 {
                                        parent
                                            .spawn((
                                                Text(format!("Item {i}")),
                                                TextFont {
                                                    font: asset_server
                                                        .load("fonts/FiraSans-Bold.ttf"),
                                                    ..default()
                                                },
                                                Label,
                                                AccessibilityNode(Accessible::new(Role::ListItem)),
                                            ))
                                            .insert(Pickable {
                                                should_block_lower: false,
                                                ..default()
                                            });
                                    }
                                });
                        });
                });

            parent
                .spawn(Node {
                    left: Val::Px(210.),
                    bottom: Val::Px(10.),
                    position_type: PositionType::Absolute,
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(200.0),
                                height: Val::Px(200.0),
                                border: UiRect::all(Val::Px(20.)),
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BorderColor::all(LIME),
                            BackgroundColor(Color::srgb(0.8, 0.8, 1.)),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                ImageNode::new(asset_server.load("branding/bevy_logo_light.png")),
                                // Uses the transform to rotate the logo image by 45 degrees
                                Node {
                                    ..Default::default()
                                },
                                UiTransform {
                                    rotation: Rot2::radians(0.25 * PI),
                                    ..Default::default()
                                },
                                BorderRadius::all(Val::Px(10.)),
                                Outline {
                                    width: Val::Px(2.),
                                    offset: Val::Px(4.),
                                    color: DARK_GRAY.into(),
                                },
                            ));
                        });
                });

            let shadow_style = ShadowStyle {
                color: Color::BLACK.with_alpha(0.5),
                blur_radius: Val::Px(2.),
                x_offset: Val::Px(10.),
                y_offset: Val::Px(10.),
                ..default()
            };

            // render order test: reddest in the back, whitest in the front (flex center)
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .insert(Pickable::IGNORE)
                .with_children(|parent| {
                    parent
                        .spawn((
                            Node {
                                width: Val::Px(100.0),
                                height: Val::Px(100.0),
                                ..default()
                            },
                            BackgroundColor(Color::srgb(1.0, 0.0, 0.)),
                            BoxShadow::from(shadow_style),
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                Node {
                                    // Take the size of the parent node.
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(20.),
                                    bottom: Val::Px(20.),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(1.0, 0.3, 0.3)),
                                BoxShadow::from(shadow_style),
                            ));
                            parent.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(40.),
                                    bottom: Val::Px(40.),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(1.0, 0.5, 0.5)),
                                BoxShadow::from(shadow_style),
                            ));
                            parent.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(60.),
                                    bottom: Val::Px(60.),
                                    ..default()
                                },
                                BackgroundColor(Color::srgb(0.0, 0.7, 0.7)),
                                BoxShadow::from(shadow_style),
                            ));
                            // alpha test
                            parent.spawn((
                                Node {
                                    width: Val::Percent(100.0),
                                    height: Val::Percent(100.0),
                                    position_type: PositionType::Absolute,
                                    left: Val::Px(80.),
                                    bottom: Val::Px(80.),
                                    ..default()
                                },
                                BackgroundColor(Color::srgba(1.0, 0.9, 0.9, 0.4)),
                                BoxShadow::from(ShadowStyle {
                                    color: Color::BLACK.with_alpha(0.3),
                                    ..shadow_style
                                }),
                            ));
                        });
                });
            // bevy logo (flex center)
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexStart,
                    ..default()
                })
                .with_children(|parent| {
                    // bevy logo (image)
                    parent
                        .spawn((
                            ImageNode::new(asset_server.load("branding/bevy_logo_dark_big.png"))
                                .with_mode(NodeImageMode::Stretch),
                            Node {
                                width: Val::Px(500.0),
                                height: Val::Px(125.0),
                                margin: UiRect::top(Val::VMin(5.)),
                                ..default()
                            },
                        ))
                        .with_children(|parent| {
                            // alt text
                            // This UI node takes up no space in the layout and the `Text` component is used by the accessibility module
                            // and is not rendered.
                            parent.spawn((
                                Node {
                                    display: Display::None,
                                    ..default()
                                },
                                Text::new("Bevy logo"),
                            ));
                        });
                });

            // four bevy icons demonstrating image flipping
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    position_type: PositionType::Absolute,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::FlexEnd,
                    column_gap: Val::Px(10.),
                    padding: UiRect::all(Val::Px(10.)),
                    ..default()
                })
                .insert(Pickable::IGNORE)
                .with_children(|parent| {
                    for (flip_x, flip_y) in
                        [(false, false), (false, true), (true, true), (true, false)]
                    {
                        parent.spawn((
                            ImageNode {
                                image: asset_server.load("branding/icon.png"),
                                flip_x,
                                flip_y,
                                ..default()
                            },
                            Node {
                                // The height will be chosen automatically to preserve the image's aspect ratio
                                width: Val::Px(75.),
                                ..default()
                            },
                        ));
                    }
                });
        });
}

#[cfg(feature = "bevy_ui_debug")]
// The system that will enable/disable the debug outlines around the nodes
fn toggle_debug_overlay(
    input: Res<ButtonInput<KeyCode>>,
    mut debug_options: ResMut<UiDebugOptions>,
    mut root_node_query: Query<&mut Visibility, (With<Node>, Without<ChildOf>)>,
) {
    info_once!("The debug outlines are enabled, press Space to turn them on/off");
    if input.just_pressed(KeyCode::Space) {
        // The toggle method will enable the debug overlay if disabled and disable if enabled
        debug_options.toggle();
    }

    if input.just_pressed(KeyCode::KeyS) {
        // Toggle debug outlines for nodes with `ViewVisibility` set to false.
        debug_options.show_hidden = !debug_options.show_hidden;
    }

    if input.just_pressed(KeyCode::KeyC) {
        // Toggle outlines for clipped UI nodes.
        debug_options.show_clipped = !debug_options.show_clipped;
    }

    if input.just_pressed(KeyCode::KeyV) {
        for mut visibility in root_node_query.iter_mut() {
            // Toggle the UI root node's visibility
            visibility.toggle_inherited_hidden();
        }
    }
}

/// Updates the scroll position of scrollable nodes in response to mouse input
pub fn update_scroll_position(
    mut mouse_wheel_events: EventReader<MouseWheel>,
    hover_map: Res<HoverMap>,
    mut scrolled_node_query: Query<(&mut ScrollPosition, &ComputedNode), Without<CoreScrollbar>>,
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
                if let Ok((mut scroll_position, scroll_content)) =
                    scrolled_node_query.get_mut(*entity)
                {
                    let visible_size = scroll_content.size();
                    let content_size = scroll_content.content_size();

                    let range = (content_size.y - visible_size.y).max(0.)
                        * scroll_content.inverse_scale_factor;

                    scroll_position.x -= dx;
                    scroll_position.y = (scroll_position.y - dy).clamp(0., range);
                }
            }
        }
    }
}
