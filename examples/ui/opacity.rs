//! Demonstrates how opacity with a hierarchy works

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // root node
    commands.spawn(root()).with_children(|parent| {
        parent.spawn(holder(1.0)).with_children(|parent| {
            parent.spawn(text("Fully Opaque"));
            parent.spawn(bar(Color::GREEN));
        });
        parent.spawn(holder(0.5)).with_children(|parent| {
            parent.spawn(text("Half Opaque"));
            parent.spawn(bar(Color::RED));
            parent.spawn(holder(1.0)).with_children(|parent| {
                parent.spawn(text("Half Opaque"));
                parent.spawn(bar(Color::PURPLE));
            });
            parent.spawn(holder(0.5)).with_children(|parent| {
                parent.spawn(text("Quarter Opaque"));
                parent.spawn(bar(Color::BLUE));
            });
        });
    });
}

fn root() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        },
        Name::new("main"),
    )
}

fn holder(opacity: f32) -> impl Bundle {
    (NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            margin: UiRect {
                bottom: Val::Px(10.),
                ..default()
            },
            row_gap: Val::Px(10.),
            padding: UiRect::all(Val::Px(18.)),
            ..default()
        },
        background_color: Color::rgba(0.08, 0.08, 0.11, 1.0).into(),
        opacity: Opacity(opacity),
        ..default()
    },)
}

fn bar(color: Color) -> impl Bundle {
    (NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            justify_content: JustifyContent::FlexStart,
            margin: UiRect {
                bottom: Val::Px(10.),
                ..default()
            },
            row_gap: Val::Px(10.),
            padding: UiRect::all(Val::Px(18.)),
            ..default()
        },
        background_color: color.into(),
        ..default()
    },)
}

fn text(text: impl Into<String>) -> impl Bundle {
    (TextBundle {
        text: Text::from_section(
            text.into(),
            TextStyle {
                font_size: 30.0,
                color: Color::WHITE,
                ..Default::default()
            },
        ),
        ..default()
    },)
}
