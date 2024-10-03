//! Example demonstrating bordered UI nodes

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    let root = commands
        .spawn(NodeBundle {
            style: Style {
                margin: UiRect::all(Val::Px(25.0)),
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..Default::default()
            },
            background_color: Color::srgb(0.25, 0.25, 0.25).into(),
            ..Default::default()
        })
        .id();

    let root_rounded = commands
        .spawn(NodeBundle {
            style: Style {
                margin: UiRect::all(Val::Px(25.0)),
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..Default::default()
            },
            background_color: Color::srgb(0.25, 0.25, 0.25).into(),
            ..Default::default()
        })
        .id();

    // labels for the different border edges
    let border_labels = [
        "None",
        "All",
        "Left",
        "Right",
        "Top",
        "Bottom",
        "Horizontal",
        "Vertical",
        "Top Left",
        "Bottom Left",
        "Top Right",
        "Bottom Right",
        "Top Bottom Right",
        "Top Bottom Left",
        "Top Left Right",
        "Bottom Left Right",
    ];

    // all the different combinations of border edges
    // these correspond to the labels above
    let borders = [
        UiRect::default(),
        UiRect::all(Val::Px(10.)),
        UiRect::left(Val::Px(10.)),
        UiRect::right(Val::Px(10.)),
        UiRect::top(Val::Px(10.)),
        UiRect::bottom(Val::Px(10.)),
        UiRect::horizontal(Val::Px(10.)),
        UiRect::vertical(Val::Px(10.)),
        UiRect {
            left: Val::Px(20.),
            top: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Val::Px(10.),
            bottom: Val::Px(20.),
            ..Default::default()
        },
        UiRect {
            right: Val::Px(20.),
            top: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            right: Val::Px(10.),
            bottom: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            right: Val::Px(10.),
            top: Val::Px(20.),
            bottom: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Val::Px(10.),
            top: Val::Px(10.),
            bottom: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Val::Px(20.),
            right: Val::Px(10.),
            top: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Val::Px(10.),
            right: Val::Px(10.),
            bottom: Val::Px(20.),
            ..Default::default()
        },
    ];

    for (label, border) in border_labels.into_iter().zip(borders) {
        let inner_spot = commands
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Px(10.),
                    height: Val::Px(10.),
                    ..Default::default()
                },
                background_color: YELLOW.into(),
                ..Default::default()
            })
            .id();
        let border_node = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(50.),
                        height: Val::Px(50.),
                        border,
                        margin: UiRect::all(Val::Px(20.)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    background_color: MAROON.into(),
                    border_color: RED.into(),
                    ..Default::default()
                },
                Outline {
                    width: Val::Px(6.),
                    offset: Val::Px(6.),
                    color: Color::WHITE,
                },
            ))
            .add_child(inner_spot)
            .id();
        let label_node = commands
            .spawn((
                TextNEW::new(label),
                TextStyle {
                    font_size: 9.0,
                    ..Default::default()
                },
            ))
            .id();
        let container = commands
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                ..Default::default()
            })
            .add_children(&[border_node, label_node])
            .id();
        commands.entity(root).add_child(container);
    }

    for (label, border) in border_labels.into_iter().zip(borders) {
        let inner_spot = commands
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Px(10.),
                    height: Val::Px(10.),
                    ..Default::default()
                },
                border_radius: BorderRadius::MAX,
                background_color: YELLOW.into(),
                ..Default::default()
            })
            .id();
        let non_zero = |x, y| x != Val::Px(0.) && y != Val::Px(0.);
        let border_size = |x, y| if non_zero(x, y) { f32::MAX } else { 0. };
        let border_radius = BorderRadius::px(
            border_size(border.left, border.top),
            border_size(border.right, border.top),
            border_size(border.right, border.bottom),
            border_size(border.left, border.bottom),
        );
        let border_node = commands
            .spawn((
                NodeBundle {
                    style: Style {
                        width: Val::Px(50.),
                        height: Val::Px(50.),
                        border,
                        margin: UiRect::all(Val::Px(20.)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    background_color: MAROON.into(),
                    border_color: RED.into(),
                    border_radius,
                    ..Default::default()
                },
                Outline {
                    width: Val::Px(6.),
                    offset: Val::Px(6.),
                    color: Color::WHITE,
                },
            ))
            .add_child(inner_spot)
            .id();
        let label_node = commands
            .spawn((
                TextNEW::new(label),
                TextStyle {
                    font_size: 9.0,
                    ..Default::default()
                },
            ))
            .id();
        let container = commands
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                ..Default::default()
            })
            .add_children(&[border_node, label_node])
            .id();
        commands.entity(root_rounded).add_child(container);
    }

    let border_label = commands
        .spawn(NodeBundle {
            style: Style {
                margin: UiRect {
                    left: Val::Px(25.0),
                    right: Val::Px(25.0),
                    top: Val::Px(25.0),
                    bottom: Val::Px(0.0),
                },
                ..Default::default()
            },
            background_color: Color::srgb(0.25, 0.25, 0.25).into(),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn((
                TextNEW::new("Borders"),
                TextStyle {
                    font_size: 20.0,
                    ..Default::default()
                },
            ));
        })
        .id();

    let border_rounded_label = commands
        .spawn(NodeBundle {
            style: Style {
                margin: UiRect {
                    left: Val::Px(25.0),
                    right: Val::Px(25.0),
                    top: Val::Px(25.0),
                    bottom: Val::Px(0.0),
                },
                ..Default::default()
            },
            background_color: Color::srgb(0.25, 0.25, 0.25).into(),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn((
                TextNEW::new("Borders Rounded"),
                TextStyle {
                    font_size: 20.0,
                    ..Default::default()
                },
            ));
        })
        .id();

    commands
        .spawn(NodeBundle {
            style: Style {
                margin: UiRect::all(Val::Px(25.0)),
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..Default::default()
            },
            background_color: Color::srgb(0.25, 0.25, 0.25).into(),
            ..Default::default()
        })
        .add_child(border_label)
        .add_child(root)
        .add_child(border_rounded_label)
        .add_child(root_rounded);
}
