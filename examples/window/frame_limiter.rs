//! Demonstrates capping Bevy's frame rate in the `winit` event loop.
//!
//! Press space to toggle the limiter and up and down to change the target FPS.
//! The on-screen text shows the requested limit alongside the measured smoothed FPS.

use bevy::{
    color::palettes::basic::{LIME, YELLOW},
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
    winit::WinitSettings,
};

const DEFAULT_FPS: u16 = 60;
const MIN_FPS: u16 = 30;
const MAX_FPS: u16 = 240;
const FPS_STEP: u16 = 30;

fn main() {
    App::new()
        .insert_resource(FrameLimiterSettings::default())
        .insert_resource(WinitSettings::game_with_max_fps(DEFAULT_FPS as f64))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Frame limiter".into(),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                adjust_frame_limiter,
                apply_frame_limiter,
                rotate_cube,
                update_overlay,
            ),
        )
        .run();
}

#[derive(Resource, Debug, Clone)]
struct FrameLimiterSettings {
    enabled: bool,
    max_fps: u16,
}

impl Default for FrameLimiterSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            max_fps: DEFAULT_FPS,
        }
    }
}

impl FrameLimiterSettings {
    fn winit_settings(&self) -> WinitSettings {
        if self.enabled {
            WinitSettings::game_with_max_fps(self.max_fps as f64)
        } else {
            WinitSettings::game()
        }
    }

    fn limiter_label(&self) -> String {
        if self.enabled {
            format!("capped at {} FPS", self.max_fps)
        } else {
            "uncapped".to_string()
        }
    }
}

#[derive(Component)]
struct Rotator;

#[derive(Component)]
struct OverlayText;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.7, 0.7, 0.7))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Rotator,
    ));

    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(1.0, 1.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Text::default(),
        Node {
            align_self: AlignSelf::FlexStart,
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        OverlayText,
        children![
            TextSpan::new("Space: toggle limiter | Up/Down: target FPS\n"),
            (TextSpan::default(), TextColor(LIME.into())),
            (TextSpan::new("\nSmoothed FPS: "), TextColor(YELLOW.into())),
            (TextSpan::new(""), TextColor(YELLOW.into())),
        ],
    ));
}

fn adjust_frame_limiter(
    input: Res<ButtonInput<KeyCode>>,
    mut settings: ResMut<FrameLimiterSettings>,
) {
    let mut changed = false;

    if input.just_pressed(KeyCode::Space) {
        settings.enabled = !settings.enabled;
        changed = true;
    }

    if input.just_pressed(KeyCode::ArrowUp) {
        settings.max_fps = (settings.max_fps + FPS_STEP).min(MAX_FPS);
        settings.enabled = true;
        changed = true;
    }

    if input.just_pressed(KeyCode::ArrowDown) {
        settings.max_fps = settings.max_fps.saturating_sub(FPS_STEP).max(MIN_FPS);
        settings.enabled = true;
        changed = true;
    }

    if changed {
        info!("Frame limiter updated: {}", settings.limiter_label());
    }
}

fn apply_frame_limiter(
    settings: Res<FrameLimiterSettings>,
    mut winit_settings: ResMut<WinitSettings>,
    mut window: Single<&mut Window>,
) {
    if !settings.is_changed() {
        return;
    }

    *winit_settings = settings.winit_settings();
    window.title = format!("Frame limiter | {}", settings.limiter_label());
}

fn rotate_cube(time: Res<Time>, mut cube_transform: Query<&mut Transform, With<Rotator>>) {
    for mut transform in &mut cube_transform {
        transform.rotate_x(time.delta_secs());
        transform.rotate_local_y(time.delta_secs());
    }
}

fn update_overlay(
    settings: Res<FrameLimiterSettings>,
    diagnostics: Res<DiagnosticsStore>,
    text: Single<Entity, With<OverlayText>>,
    mut writer: TextUiWriter,
) {
    *writer.text(*text, 1) = format!("Mode: {}", settings.limiter_label(),);

    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(bevy::diagnostic::Diagnostic::smoothed)
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "--".to_string());
    *writer.text(*text, 3) = fps;
}
