//! UI text benchmark

use argh::FromArgs;
use bevy::{
    color::palettes::css::ORANGE_RED,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    text::TextColor,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

const FONT_SIZE: FontSize = FontSize::Px(7.0);

#[derive(FromArgs, Resource)]
struct Args {
    /// At the start of each frame despawn any existing UI nodes and spawn a new UI tree
    #[argh(switch)]
    respawn: bool,
}

/// This example shows what happens when there is a lot of buttons on screen.
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
    .add_systems(Startup, setup)
    .add_systems(Update, update);
}

fn column() -> Bundle {}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let root = commands
        .spawn((Node {
            display: Display::Grid,
        },))
        .id();

    for font_path in [
        "EBGaramond12-Regular.otf",
        "FiraMono-Medium.ttf",
        "FiraSans-Bold.ttf",
        "MonaSans-VariableFont.ttf",
    ] {
        let font = asset_server.load(format!("fonts/{font_path}"));
    }
}
