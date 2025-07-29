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
            ..default()
        },
        UiRect {
            left: Val::Px(10.),
            bottom: Val::Px(20.),
            ..default()
        },
        UiRect {
            right: Val::Px(20.),
            top: Val::Px(10.),
            ..default()
        },
        UiRect {
            right: Val::Px(10.),
            bottom: Val::Px(10.),
            ..default()
        },
        UiRect {
            right: Val::Px(10.),
            top: Val::Px(20.),
            bottom: Val::Px(10.),
            ..default()
        },
        UiRect {
            left: Val::Px(10.),
            top: Val::Px(10.),
            bottom: Val::Px(10.),
            ..default()
        },
        UiRect {
            left: Val::Px(20.),
            right: Val::Px(10.),
            top: Val::Px(10.),
            ..default()
        },
        UiRect {
            left: Val::Px(10.),
            right: Val::Px(10.),
            bottom: Val::Px(20.),
            ..default()
        },
    ];

    let borders_examples = (
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
                                width: Val::Px(50.),
                                height: Val::Px(50.),
                                border,
                                margin: UiRect::all(Val::Px(20.)),
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
                                width: Val::Px(6.),
                                offset: Val::Px(6.),
                                color: Color::WHITE,
                            },
                            children![(
                                Node {
                                    width: Val::Px(10.),
                                    height: Val::Px(10.),
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

    let non_zero = |x, y| x != Val::Px(0.) && y != Val::Px(0.);
    let border_size = move |x, y| {
        if non_zero(x, y) {
            f32::MAX
        } else {
            0.
        }
    };

    let borders_examples_rounded = (
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
        Children::spawn(SpawnIter(border_labels.into_iter().zip(borders).map(
            move |(label, border)| {
                (
                    Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    children![
                        (
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
                            BorderColor {
                                top: RED.into(),
                                bottom: YELLOW.into(),
                                left: GREEN.into(),
                                right: BLUE.into(),
                            },
                            BorderRadius::px(
                                border_size(border.left, border.top),
                                border_size(border.right, border.top),
                                border_size(border.right, border.bottom,),
                                border_size(border.left, border.bottom),
                            ),
                            Outline {
                                width: Val::Px(6.),
                                offset: Val::Px(6.),
                                color: Color::WHITE,
                            },
                            children![(
                                Node {
                                    width: Val::Px(10.),
                                    height: Val::Px(10.),
                                    ..default()
                                },
                                BorderRadius::MAX,
                                BackgroundColor(YELLOW.into()),
                            )],
                        ),
                        (Text::new(label), TextFont::from_font_size(9.0))
                    ],
                )
            },
        ))),
    );

    commands.spawn((
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
            margin: UiRect::all(Val::Px(25.0)),
            ..default()
        },
        children![(Text::new(text), TextFont::from_font_size(20.0))],
    )
}
