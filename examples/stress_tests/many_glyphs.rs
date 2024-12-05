//! Simple text rendering benchmark.
//!
//! Creates a text block with a single span containing `100_000` glyphs,
//! and renders it with the UI in a white color and with Text2d in a red color.
//!
//! To recompute all text each frame run
//! `cargo run --example many_glyphs --release recompute-text`
use bevy::{
    color::palettes::basic::RED,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{LineBreak, TextBounds},
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0),
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
    .add_systems(Startup, setup);

    if std::env::args().any(|arg| arg == "recompute-text") {
        app.add_systems(Update, force_text_recomputation);
    }

    app.run();
}

fn setup(mut commands: Commands) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera2d);
    let text_string = "0123456789".repeat(10_000);
    let text_font = TextFont {
        font_size: 4.,
        ..Default::default()
    };
    let text_block = TextLayout {
        justify: JustifyText::Left,
        linebreak: LineBreak::AnyCharacter,
    };

    commands
        .spawn(Node {
            width: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        })
        .with_children(|commands| {
            commands
                .spawn(Node {
                    width: Val::Px(1000.),
                    ..Default::default()
                })
                .with_child((Text(text_string.clone()), text_font.clone(), text_block));
        });

    commands.spawn((
        Text2d::new(text_string),
        TextColor(RED.into()),
        bevy::sprite::Anchor::Center,
        TextBounds::new_horizontal(1000.),
        text_block,
    ));
}

fn force_text_recomputation(mut text_query: Query<&mut TextLayout>) {
    for mut block in &mut text_query {
        block.set_changed();
    }
}
