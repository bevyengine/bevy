//! Text pipeline benchmark.
//!
//! Continuously recomputes a large block of text with 100 text spans.

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

<<<<<<< HEAD
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
=======
    commands.spawn(Camera2d);

    let make_spans = |i| {
        [
            (
                "text".repeat(i),
                TextStyle {
                    font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                    font_size: (4 + i % 10) as f32,
                    color: BLUE.into(),
                    ..Default::default()
>>>>>>> 4f86a970a (Update text stress tests (#4))
                },
            ),
            (
                "pipeline".repeat(i),
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: (4 + i % 11) as f32,
                    color: YELLOW.into(),
                    ..default()
                },
            ),
        ]
    };

    let [t1, p1] = make_spans(1);
    commands
        .spawn((
            Text2d::new(t1.0),
            t1.1,
            TextBlock {
                justify: JustifyText::Center,
                linebreak: LineBreak::AnyCharacter,
                ..Default::default()
            },
            TextBounds::default(),
        ))
        .with_children(|parent| {
            parent.spawn((TextSpan2d(p1.0), p1.1));
            for i in 2..=50 {
                let [t, p] = make_spans(i);
                parent.spawn((TextSpan2d(t.0), t.1));
                parent.spawn((TextSpan2d(p.0), p.1));
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
