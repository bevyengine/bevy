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
        Border::default(),
        Border::all(Num::Px(10.)),
        Border::left(Num::Px(10.)),
        Border::right(Num::Px(10.)),
        Border::top(Num::Px(10.)),
        Border::bottom(Num::Px(10.)),
        Border::horizontal(Num::Px(10.)),
        Border::vertical(Num::Px(10.)),
        Border {
            left: Num::Px(10.),
            top: Num::Px(10.),
            ..Default::default()
        },
        Border {
            left: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        Border {
            right: Num::Px(10.),
            top: Num::Px(10.),
            ..Default::default()
        },
        Border {
            right: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        Border {
            right: Num::Px(10.),
            top: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        Border {
            left: Num::Px(10.),
            top: Num::Px(10.),
            bottom: Num::Px(10.),
            ..Default::default()
        },
        Border {
            left: Num::Px(10.),
            right: Num::Px(10.),
            top: Num::Px(10.),
            ..Default::default()
        },
        Border {
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
                    width: Val::Px(10.),
                    height: Val::Px(10.),
                    ..Default::default()
                },
                background_color: Color::YELLOW.into(),
                ..Default::default()
            })
            .id();
        let bordered_node = commands
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Px(50.),
                    height: Val::Px(50.),
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
