//! Example demonstrating bordered UI nodes

use bevy::prelude::*;

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
                flex_basis: Num::Percent(100.0),
                margin: Margin::all(Num::Px(25.0)),
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        })
        .id();

    // all the different combinations of border edges
    let borders = [
        UiRect::default(),
        UiRect::all(Num::Px(10.)),
        UiRect::left(Num::Px(10.)),
        UiRect::right(Num::Px(10.)),
        UiRect::top(Num::Px(10.)),
        UiRect::bottom(Num::Px(10.)),
        UiRect::horizontal(Num::Px(10.)),
        UiRect::vertical(Num::Px(10.)),
        UiRect {
            left: Num::Px(10.),
            top: Num::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        UiRect {
            right: Num::Px(10.),
            top: Num::Px(10.),
            ..Default::default()
        },
        UiRect {
            right: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        UiRect {
            right: Num::Px(10.),
            top: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Num::Px(10.),
            top: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Num::Px(10.),
            right: Num::Px(10.),
            top: Num::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Num::Px(10.),
            right: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
    ];

    for i in 0..64 {
        let inner_spot = commands
            .spawn(NodeBundle {
                style: Style {
                    width: Num::Px(10.),
                    height: Num::Px(10.),
                    ..Default::default()
                },
                background_color: Color::YELLOW.into(),
                ..Default::default()
            })
            .id();
        let bordered_node = commands
            .spawn(NodeBundle {
                style: Style {
                    width: Num::Px(50.),
                    height: Num::Px(50.),
                    border: borders[i % borders.len()],
                    margin: Margin::all(Num::Px(2.)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..Default::default()
                },
                background_color: Color::BLUE.into(),
                border_color: Color::WHITE.with_a(0.5).into(),
                ..Default::default()
            })
            .add_child(inner_spot)
            .id();
        commands.entity(root).add_child(bordered_node);
    }
}
