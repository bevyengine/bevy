//! Text pipeline benchmark.
//!
//! Continuously recomputes a large `Text` component with 100 sections.

use bevy::{
    color::palettes::basic::{BLUE, YELLOW},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{LineBreak, TextBounds},
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(1920.0, 1080.0)
                        .with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .add_systems(Startup, spawn)
        .add_systems(Update, update_text_bounds)
        .run();
}

fn spawn(mut commands: Commands, asset_server: Res<AssetServer>) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera2d);
    let sections = (1..=50)
        .flat_map(|i| {
            [
                TextSection {
                    value: "text".repeat(i),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                        font_size: (4 + i % 10) as f32,
                        color: BLUE.into(),
                    },
                },
                TextSection {
                    value: "pipeline".repeat(i),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: (4 + i % 11) as f32,
                        color: YELLOW.into(),
                    },
                },
            ]
        })
        .collect::<Vec<_>>();
    commands.spawn(Text2dBundle {
        text: Text {
            sections,
            justify: JustifyText::Center,
            linebreak: LineBreak::AnyCharacter,
            ..default()
        },
        ..Default::default()
    });
}

// changing the bounds of the text will cause a recomputation
fn update_text_bounds(time: Res<Time>, mut text_bounds_query: Query<&mut TextBounds>) {
    let width = (1. + ops::sin(time.elapsed_seconds())) * 600.0;
    for mut text_bounds in text_bounds_query.iter_mut() {
        text_bounds.width = Some(width);
    }
}
