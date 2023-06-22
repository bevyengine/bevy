//! Simple text rendering benchmark.
//!
//! Creates a `Text` with a single `TextSection` containing `100_000` glyphs,
//! and renders it with the UI in a white color and with Text2d in a red color.
//!
//! To recompute all text each frame run
//! `cargo run --example many_glyphs --release recompute-text`
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{BreakLineOn, Text2dBounds},
    window::{PresentMode, WindowPlugin},
};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::Immediate,
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
    ))
    .add_systems(Startup, setup);

    if std::env::args().any(|arg| arg == "recompute-text") {
        app.add_systems(Update, force_text_recomputation);
    }

    app.run();
}

fn setup(mut commands: Commands) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera2dBundle::default());
    let mut text = Text {
        sections: vec![TextSection {
            value: "0123456789".repeat(10_000),
            style: TextStyle {
                font_size: 4.,
                color: Color::WHITE,
                ..default()
            },
        }],
        alignment: TextAlignment::Left,
        linebreak_behavior: BreakLineOn::AnyCharacter,
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                flex_basis: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|commands| {
            commands.spawn(TextBundle {
                text: text.clone(),
                style: Style {
                    width: Val::Px(1000.),
                    ..Default::default()
                },
                ..Default::default()
            });
        });

    text.sections[0].style.color = Color::RED;

    commands.spawn(Text2dBundle {
        text,
        text_anchor: bevy::sprite::Anchor::Center,
        text_2d_bounds: Text2dBounds {
            size: Vec2::new(1000., f32::INFINITY),
        },
        ..Default::default()
    });
}

fn force_text_recomputation(mut text_query: Query<&mut Text>) {
    for mut text in &mut text_query {
        text.set_changed();
    }
}
