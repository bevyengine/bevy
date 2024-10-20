//! Example demonstrating bordered UI nodes

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);
    let root = commands
        .spawn((
            Node {
                margin: UiRect::all(Val::Px(25.0)),
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .id();

    let root_rounded = commands
        .spawn((
            Node {
                margin: UiRect::all(Val::Px(25.0)),
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        ))
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
            .spawn((
                Node {
                    width: Val::Px(10.),
                    height: Val::Px(10.),
                    ..default()
                },
                BackgroundColor(YELLOW.into()),
            ))
            .id();
        let border_node = commands
            .spawn((
                Node {
                    width: Val::Px(50.),
                    height: Val::Px(50.),
                    border,
                    margin: UiRect::all(Val::Px(20.)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(MAROON.into()),
                BorderColor(RED.into()),
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
                Text::new(label),
                TextFont {
                    font_size: 9.0,
                    ..Default::default()
                },
            ))
            .id();
        let container = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .add_children(&[border_node, label_node])
            .id();
        commands.entity(root).add_child(container);
    }

    for (label, border) in border_labels.into_iter().zip(borders) {
        let inner_spot = commands
            .spawn((
                Node {
                    width: Val::Px(10.),
                    height: Val::Px(10.),
                    ..default()
                },
                BorderRadius::MAX,
                BackgroundColor(YELLOW.into()),
            ))
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
                Node {
                    width: Val::Px(50.),
                    height: Val::Px(50.),
                    border,
                    margin: UiRect::all(Val::Px(20.)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                BackgroundColor(MAROON.into()),
                BorderColor(RED.into()),
                border_radius,
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
                Text::new(label),
                TextFont {
                    font_size: 9.0,
                    ..Default::default()
                },
            ))
            .id();
        let container = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                ..default()
            })
            .add_children(&[border_node, label_node])
            .id();
        commands.entity(root_rounded).add_child(container);
    }

    let border_label = commands
        .spawn((
            Node {
                margin: UiRect {
                    left: Val::Px(25.0),
                    right: Val::Px(25.0),
                    top: Val::Px(25.0),
                    bottom: Val::Px(0.0),
                },
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text::new("Borders"),
                TextFont {
                    font_size: 20.0,
                    ..Default::default()
                },
            ));
        })
        .id();

    let border_rounded_label = commands
        .spawn((
            Node {
                margin: UiRect {
                    left: Val::Px(25.0),
                    right: Val::Px(25.0),
                    top: Val::Px(25.0),
                    bottom: Val::Px(0.0),
                },
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text::new("Borders Rounded"),
                TextFont {
                    font_size: 20.0,
                    ..Default::default()
                },
            ));
        })
        .id();

    commands
        .spawn((
            Node {
                margin: UiRect::all(Val::Px(25.0)),
                flex_direction: FlexDirection::Column,
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..default()
            },
            BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        ))
        .add_child(border_label)
        .add_child(root)
        .add_child(border_rounded_label)
        .add_child(root_rounded);
}
