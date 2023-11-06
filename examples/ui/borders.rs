//! Example demonstrating bordered UI nodes

use bevy::{prelude::*, ui::{style, node_bundle, outline}};
use bevy_internal::ui::ui_rect;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    let root = commands
        .spawn(node_bundle!(
            style: style!(
                margin: UiRect::all(Val::Px(25.0)),
                align_self: AlignSelf::Stretch,
                justify_self: JustifySelf::Stretch,
                flex_wrap: FlexWrap::Wrap,
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                align_content: AlignContent::FlexStart
            ),
            background_color: BackgroundColor(Color::DARK_GRAY)
        ))
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
        ui_rect!(
            left: Val::Px(10.),
            top: Val::Px(10.)
        ),
        ui_rect!(
            left: Val::Px(10.),
            bottom: Val::Px(10.)
        ),
        ui_rect!(
            right: Val::Px(10.),
            top: Val::Px(10.)
        ),
        ui_rect!(
            right: Val::Px(10.),
            bottom: Val::Px(10.)
        ),
        ui_rect!(
            right: Val::Px(10.),
            top: Val::Px(10.),
            bottom: Val::Px(10.)
        ),
        ui_rect!(
            left: Val::Px(10.),
            top: Val::Px(10.),
            bottom: Val::Px(10.)
        ),
        ui_rect!(
            left: Val::Px(10.),
            right: Val::Px(10.),
            top: Val::Px(10.)
        ),
        ui_rect!(
            left: Val::Px(10.),
            right: Val::Px(10.),
            bottom: Val::Px(10.)
        )
    ];

    for i in 0..64 {
        let inner_spot = commands
            .spawn(node_bundle!(
                style: style!(
                    width: Val::Px(10.),
                    height: Val::Px(10.)
                ),
                background_color: Color::YELLOW.into()
            ))
            .id();
        let bordered_node = commands
            .spawn((
                node_bundle!(
                    style: style!(
                        width: Val::Px(50.),
                        height: Val::Px(50.),
                        border: borders[i % borders.len()],
                        margin: UiRect::all(Val::Px(20.)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center
                    ),
                    background_color: Color::MAROON.into(),
                    border_color: Color::RED.into()
                ),
                outline!(
                    width: Val::Px(6.),
                    offset: Val::Px(6.),
                    color: Color::WHITE
                ),
            ))
            .add_child(inner_spot)
            .id();
        commands.entity(root).add_child(bordered_node);
    }
}
