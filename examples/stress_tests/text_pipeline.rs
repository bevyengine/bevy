//! Text pipeline benchmark.
//!
//! Continuously recomputes a large `Text` component with 100 sections.

use bevy::{
    color::palettes::basic::{BLUE, YELLOW},
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{BreakLineOn, Text2dBounds},
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

    commands.spawn(Camera2dBundle::default());
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
