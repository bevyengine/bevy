//! Example demonstrating text gizmos.

use bevy::color::palettes::css::{BLUE, GREEN, RED, YELLOW};
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::math::Isometry2d;
use bevy::prelude::*;
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::WinitSettings;

const TEXT_COUNT: usize = 50;
const START_X: f32 = -700.0;
const START_Y: f32 = 200.0;
const X_STEP: f32 = 300.0;
const Y_STEP: f32 = 50.0;

fn main() {
    App::new()
        .insert_resource(WinitSettings::continuous())
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, draw_labels)
        .run();
}

fn setup(mut commands: Commands, mut gizmo_config_store: ResMut<GizmoConfigStore>) {
    commands.spawn(Camera2d);

    let (config, _) = gizmo_config_store.config_mut::<DefaultGizmoConfigGroup>();

    config.line.width = 1.;
}

fn draw_labels(mut text_gizmos: Gizmos, diagnostic: Res<DiagnosticsStore>) {
    let colors = [RED, GREEN, BLUE, YELLOW];
    for i in 0..TEXT_COUNT {
        let row = i / 5;
        let col = i % 5;
        let color = colors[i % 4];
        text_gizmos.text_2d(
            Isometry2d {
                translation: Vec2::new(
                    START_X + col as f32 * X_STEP,
                    START_Y - row as f32 * Y_STEP,
                ),
                rotation: Rot2::degrees(2.),
            },
            &format!("label {i}"),
            25.,
            color,
        );
    }

    if let Some(fps) = diagnostic.get(&FrameTimeDiagnosticsPlugin::FPS)
        && let Some(fps_smoothed) = fps.smoothed()
    {
        let pos = 0.5 * Vec2::new(1920., 1080.) - Vec2::new(X_STEP, Y_STEP);
        text_gizmos.text_2d(
            Isometry2d::from_translation(pos),
            &format!("fps: {:.1}", fps_smoothed),
            25.,
            Color::WHITE,
        );
    }

    text_gizmos.text_2d(
        Isometry2d::from_translation(Vec2::new(-200., 500.)),
        "lxgh",
        150.,
        Color::WHITE,
    );
}
