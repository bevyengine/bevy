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
                flex_basis: Val::Percent(100.0),
                margin: UiRect::all(Val::Px(25.0)),
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
        UiRect::all(Val::Px(10.)),
        UiRect::left(Val::Px(10.)),
        UiRect::right(Val::Px(10.)),
        UiRect::top(Val::Px(10.)),
        UiRect::bottom(Val::Px(10.)),
        UiRect::horizontal(Val::Px(10.)),
        UiRect::vertical(Val::Px(10.)),
        UiRect {
            left: Val::Px(10.),
            top: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Val::Px(10.),
            bottom: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            right: Val::Px(10.),
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
            top: Val::Px(10.),
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
            left: Val::Px(10.),
            right: Val::Px(10.),
            top: Val::Px(10.),
            ..Default::default()
        },
        UiRect {
            left: Val::Px(10.),
            right: Val::Px(10.),
            bottom: Val::Px(10.),
            ..Default::default()
        },
    ];

    for i in 0..64 {
        let inner_spot = commands
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Px(10.),
                    height: Val::Px(10.),
                    border_radius: UiBorderRadius::all(Val::Px(5.)),
                    ..Default::default()
                },
                background_color: Color::YELLOW.into(),
                ..Default::default()
            })
            .id();
        let border = borders[i % borders.len()];
        let border_radius = UiBorderRadius::px(
            if border.left != Val::Px(0.) && border.top != Val::Px(0.) {
                f32::MAX
            } else {
                0.
            },
            if border.right != Val::Px(0.) && border.top != Val::Px(0.) {
                f32::MAX
            } else {
                0.
            },
            if border.right != Val::Px(0.) && border.bottom != Val::Px(0.) {
                f32::MAX
            } else {
                0.
            },
            if border.left != Val::Px(0.) && border.bottom != Val::Px(0.) {
                f32::MAX
            } else {
                0.
            },
        );
        let bordered_node = commands
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Px(50.),
                    height: Val::Px(50.),
                    border,
                    margin: UiRect::all(Val::Px(2.)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border_radius,
                    ..Default::default()
                },
                background_color: Color::MAROON.into(),
                border_color: Color::CRIMSON.into(),
                ..Default::default()
            })
            .add_child(inner_spot)
            .id();
        commands.entity(root).add_child(bordered_node);
    }
}
