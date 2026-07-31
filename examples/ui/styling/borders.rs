//! Example demonstrating bordered UI nodes

use bevy::{color::palettes::css::*, ecs::spawn::SpawnIter, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

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
        UiRect::all(px(10)),
        UiRect::left(px(10)),
        UiRect::right(px(10)),
        UiRect::top(px(10)),
        UiRect::bottom(px(10)),
        UiRect::horizontal(px(10)),
        UiRect::vertical(px(10)),
        UiRect {
            left: px(20),
            top: px(10),
            ..default()
        },
        UiRect {
            left: px(10),
            bottom: px(20),
            ..default()
        },
        UiRect {
            right: px(20),
            top: px(10),
            ..default()
        },
        UiRect {
            right: px(10),
            bottom: px(10),
            ..default()
        },
        UiRect {
            right: px(10),
            top: px(20),
            bottom: px(10),
            ..default()
        },
        UiRect {
            left: px(10),
            top: px(10),
            bottom: px(10),
            ..default()
        },
        UiRect {
            left: px(20),
            right: px(10),
            top: px(10),
            ..default()
        },
        UiRect {
            left: px(10),
            right: px(10),
            bottom: px(20),
            ..default()
        },
    ];

    let borders_examples = (
        Node {
            margin: px(25).all(),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            |(label, border)| {
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
                            Node {
                                width: px(50),
                                height: px(50),
                                border,
                                margin: px(20).all(),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                ..default()
                            },
                            BackgroundColor(MAROON.into()),
                            BorderColor {
                                top: RED.into(),
                                bottom: YELLOW.into(),
                                left: GREEN.into(),
                                right: BLUE.into(),
                            },
                            Outline {
                                width: px(6),
                                offset: px(6),
                                color: Color::WHITE,
                            },
                            children![(
                                Node {
                                    width: px(10),
                                    height: px(10),
                                    ..default()
                                },
                                BackgroundColor(YELLOW.into()),
                            )]
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    let non_zero = |x, y| x != px(0) && y != px(0);
    let border_size = move |x, y| {
        if non_zero(x, y) {
            f32::MAX
        } else {
            0.
        }
    };

    let rounded_border_radii = borders.map(|border| {
        BorderRadius::px(
            border_size(border.left, border.top),
            border_size(border.right, border.top),
            border_size(border.right, border.bottom),
            border_size(border.left, border.bottom),
        )
    });

    let elliptical_border_labels = [
        "Ellipse Wide",
        "Ellipse Tall",
        "Ellipse Mixed",
        "Ellipse Percent",
    ];
    let elliptical_borders = [
        UiRect::all(px(10)),
        UiRect::all(px(10)),
        UiRect::all(px(10)),
        UiRect::all(px(10)),
    ];
    let elliptical_border_radii = [
        BorderRadius {
            top_left: Val2::new(px(25), px(8)),
            top_right: Val2::new(px(25), px(8)),
            bottom_right: Val2::new(px(25), px(8)),
            bottom_left: Val2::new(px(25), px(8)),
        },
        BorderRadius {
            top_left: Val2::new(px(8), px(25)),
            top_right: Val2::new(px(8), px(25)),
            bottom_right: Val2::new(px(8), px(25)),
            bottom_left: Val2::new(px(8), px(25)),
        },
        BorderRadius {
            top_left: Val2::new(px(25), px(12)),
            top_right: Val2::new(px(12), px(25)),
            bottom_right: Val2::new(px(25), px(12)),
            bottom_left: Val2::new(px(12), px(25)),
        },
        BorderRadius {
            top_left: Val2::new(percent(50), percent(20)),
            top_right: Val2::new(percent(20), percent(50)),
            bottom_right: Val2::new(percent(50), percent(20)),
            bottom_left: Val2::new(percent(20), percent(50)),
        },
    ];

    let borders_examples_rounded = (
        Node {
            margin: px(25).all(),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        },
        Children::spawn(SpawnIter(
            border_labels
                .into_iter()
                .zip(borders)
                .zip(rounded_border_radii)
                .map(|((label, border), border_radius)| (label, border, border_radius))
                .chain(
                    elliptical_border_labels
                        .into_iter()
                        .zip(elliptical_borders)
                        .zip(elliptical_border_radii)
                        .map(|((label, border), border_radius)| (label, border, border_radius)),
                )
                .map(move |(label, border, border_radius)| {
                    (
                        Node {
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        children![
                            (
                                Node {
                                    width: px(50),
                                    height: px(50),
                                    border,
                                    margin: px(20).all(),
                                    align_items: AlignItems::Center,
                                    justify_content: JustifyContent::Center,
                                    border_radius,
                                    ..default()
                                },
                                BackgroundColor(MAROON.into()),
                                BorderColor {
                                    top: RED.into(),
                                    bottom: YELLOW.into(),
                                    left: GREEN.into(),
                                    right: BLUE.into(),
                                },
                                Outline {
                                    width: px(6),
                                    offset: px(6),
                                    color: Color::WHITE,
                                },
                                children![(
                                    Node {
                                        width: px(10),
                                        height: px(10),
                                        border_radius: BorderRadius::MAX,
                                        ..default()
                                    },
                                    BackgroundColor(YELLOW.into()),
                                )],
                            ),
                            (Text::new(label), TextFont::from_font_size(9.0))
                        ],
                    )
                }),
        )),
    );

    commands.spawn((
        Node {
            margin: px(25).all(),
            flex_direction: FlexDirection::Column,
            align_self: AlignSelf::Stretch,
            justify_self: JustifySelf::Stretch,
            ..default()
        },
        BackgroundColor(Color::srgb(0.25, 0.25, 0.25)),
        children![
            label("Borders"),
            borders_examples,
            label("Borders Rounded"),
            borders_examples_rounded
        ],
    ));
}

// A label widget that accepts a &str and returns
// a Bundle that can be spawned
fn label(text: &str) -> impl Bundle {
    (
        Node {
            margin: px(25).all(),
            ..default()
        },
        children![(Text::new(text), TextFont::from_font_size(20.0))],
    )
}
