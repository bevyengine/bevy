//! Text pipeline benchmark.
//!
//! Continuously recomputes a large `Text` component with 100 sections.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{BreakLineOn, Text2dBounds},
    window::{PresentMode, WindowPlugin},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, spawn)
        .add_systems(Update, update_text_bounds)
        .run();
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera2dBundle::default());
    let sections = (1..=50)
        .flat_map(|i| {
            [
                TextSection {
                    value: "text".repeat(i),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: (4 + i % 10) as f32,
                        color: Color::BLUE,
                    },
                },
                TextSection {
                    value: "pipeline".repeat(i),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: (4 + i % 11) as f32,
                        color: Color::YELLOW,
                    },
                },
            ]
        })
        .collect::<Vec<_>>();
    commands.spawn(Text2dBundle {
        text: Text {
            sections,
            alignment: TextAlignment::Center,
            linebreak_behavior: BreakLineOn::AnyCharacter,
        },
        ..Default::default()
    });
}

// changing the bounds of the text will cause a recomputation
fn update_text_bounds(time: Res<Time>, mut text_bounds_query: Query<&mut Text2dBounds>) {
    let width = (1. + time.elapsed_seconds().sin()) * 600.0;
    for mut text_bounds in text_bounds_query.iter_mut() {
        text_bounds.size.x = width;
    }
}
