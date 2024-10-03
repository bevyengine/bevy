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

    commands.spawn(Camera2dBundle::default());

    let text_block = TextBlock {
        justify: JustifyText::Center,
        linebreak: LineBreak::AnyCharacter,
        ..Default::default()
    };

    let make_spans = |i| {
        [
            (
                Text2d::new("text".repeat(i)),
                TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: (4 + i % 10) as f32,
                    color: BLUE.into(),
                },
                text_block.clone(),
            ),
            (
                Text2d::new("pipeline".repeat(i)),
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: (4 + i % 11) as f32,
                    color: YELLOW.into(),
                },
                text_block.clone(),
            ),
        ]
    };

    let [t1, p1] = make_spans(1);
    commands.spawn(t1).with_children(|parent| {
        parent.spawn(p1);
        for i in 2..=50 {
            let [t, p] = make_spans(i);
            parent.spawn(t);
            parent.spawn(p);
        }
    });
}

// changing the bounds of the text will cause a recomputation
fn update_text_bounds(time: Res<Time>, mut text_bounds_query: Query<&mut TextBounds>) {
    let width = (1. + ops::sin(time.elapsed_seconds())) * 600.0;
    for mut text_bounds in text_bounds_query.iter_mut() {
        text_bounds.width = Some(width);
    }
}
