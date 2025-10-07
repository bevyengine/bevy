//! Simple text rendering benchmark.
//!
//! Creates a text block with a single span containing `100_000` glyphs,
//! and renders it with the UI in a white color and with Text2d in a red color.
//!
//! To recompute all text each frame run
//! `cargo run --example many_glyphs --release recompute-text`
use argh::FromArgs;
use bevy::{
    color::palettes::basic::RED,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::{LineBreak, TextBounds},
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

#[derive(FromArgs, Resource)]
/// `many_glyphs` stress test
struct Args {
    /// don't draw the UI text.
    #[argh(switch)]
    no_ui: bool,

    /// don't draw the Text2d text.
    #[argh(switch)]
    no_text2d: bool,

    /// whether to force the text to recompute every frame by triggering change detection.
    #[argh(switch)]
    recompute_text: bool,
}

fn main() {
    // `from_env` panics on the web
    #[cfg(not(target_arch = "wasm32"))]
    let args: Args = argh::from_env();
    #[cfg(target_arch = "wasm32")]
    let args = Args::from_args(&[], &[]).unwrap();

    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin::default(),
        LogDiagnosticsPlugin::default(),
    ))
    .insert_resource(WinitSettings::continuous())
    .add_systems(Startup, setup);

    if args.recompute_text {
        app.add_systems(Update, force_text_recomputation);
    }

    app.insert_resource(args).run();
}

fn setup(mut commands: Commands, args: Res<Args>) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera2d);
    let text_string = "0123456789".repeat(10_000);
    let text_font = TextFont {
        font_size: 4.,
        ..Default::default()
    };
    let text_block = TextLayout {
        justify: Justify::Left,
        linebreak: LineBreak::AnyCharacter,
    };

    if !args.no_ui {
        commands
            .spawn(Node {
                width: percent(100),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            })
            .with_children(|commands| {
                commands
                    .spawn(Node {
                        width: px(1000),
                        ..Default::default()
                    })
                    .with_child((Text(text_string.clone()), text_font.clone(), text_block));
            });
    }

    if !args.no_text2d {
        commands.spawn((
            Text2d::new(text_string),
            text_font.clone(),
            TextColor(RED.into()),
            bevy::sprite::Anchor::CENTER,
            TextBounds::new_horizontal(1000.),
            text_block,
        ));
    }
}

fn force_text_recomputation(mut text_query: Query<&mut TextLayout>) {
    for mut block in &mut text_query {
        block.set_changed();
    }
}
