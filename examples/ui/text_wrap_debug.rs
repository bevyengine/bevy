//! This example demonstrates text wrapping and use of the `LineBreakOn` property.

use bevy::prelude::*;
use bevy::text::BreakLineOn;
use bevy::winit::WinitSettings;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, spawn)
        .run();
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 14.0,
        color: Color::WHITE,
    };

    let root = commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.),
                ..Default::default()
            },
            background_color: Color::BLACK.into(),
            ..Default::default()
        })
        .id();

    for linebreak_behavior in [
        BreakLineOn::AnyCharacter,
        BreakLineOn::WordBoundary,
        BreakLineOn::NoWrap,
    ] {
        let row_id = commands
            .spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::SpaceAround,
                    align_items: AlignItems::Center,
                    width: Val::Percent(100.),
                    height: Val::Percent(50.),
                    ..Default::default()
                },
                ..Default::default()
            })
            .id();

        let justifications = vec![
            JustifyContent::Center,
            JustifyContent::FlexStart,
            JustifyContent::FlexEnd,
            JustifyContent::SpaceAround,
            JustifyContent::SpaceBetween,
            JustifyContent::SpaceEvenly,
        ];

        for (i, justification) in justifications.into_iter().enumerate() {
            let c = 0.3 + i as f32 * 0.1;
            let column_id = commands
                .spawn(NodeBundle {
                    style: Style {
                        justify_content: justification,
                        flex_direction: FlexDirection::Column,
                        width: Val::Percent(16.),
                        height: Val::Percent(95.),
                        overflow: Overflow::clip_x(),
                        ..Default::default()
                    },
                    background_color: Color::rgb(0.5, c, 1.0 - c).into(),
                    ..Default::default()
                })
                .id();

            let messages = [
                format!("JustifyContent::{justification:?}"),
                format!("LineBreakOn::{linebreak_behavior:?}"),
                "Line 1\nLine 2\nLine 3".to_string(),
                "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Maecenas auctor, nunc ac faucibus fringilla.".to_string(),
            ];

            for (j, message) in messages.into_iter().enumerate() {
                let text = Text {
                    sections: vec![TextSection {
                        value: message.clone(),
                        style: text_style.clone(),
                    }],
                    alignment: TextAlignment::Left,
                    linebreak_behavior,
                };
                let text_id = commands
                    .spawn(TextBundle {
                        text,
                        background_color: Color::rgb(0.8 - j as f32 * 0.2, 0., 0.).into(),
                        ..Default::default()
                    })
                    .id();
                commands.entity(column_id).add_child(text_id);
            }
            commands.entity(row_id).add_child(column_id);
        }
        commands.entity(root).add_child(row_id);
    }
}
