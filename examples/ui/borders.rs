//! Example demonstrating bordered UI nodes

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_many)
        .run();
}

fn setup_many(mut commands: Commands) {
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
        let b = commands
            .spawn(NodeBundle {
                style: Style {
                    size: Size::all(Val::Px(50.)),
                    border: borders[i % borders.len()],
                    margin: UiRect::all(Val::Px(2.)),
                    ..Default::default()
                },
                background_color: Color::BLUE.into(),
                border_style: Color::WHITE.into(),
                ..Default::default()
            })
            .id();
        commands.entity(root).add_child(b);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    let root = commands
        .spawn(NodeBundle {
            style: Style {
                flex_basis: Val::Percent(100.0),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        })
        .id();

    let b = commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::all(Val::Px(100.)),
                border: UiRect::all(Val::Px(10.)),
                ..Default::default()
            },
            background_color: Color::BLUE.into(),
            border_style: Color::WHITE.into(),
            ..Default::default()
        })
        .id();
    commands.entity(root).add_child(b);
}
