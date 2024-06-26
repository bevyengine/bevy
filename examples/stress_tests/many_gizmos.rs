//! Test rendering of many gizmos.

use std::f32::consts::TAU;

use bevy::{
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};

const SYSTEM_COUNT: u32 = 10;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Many Debug Lines".to_string(),
                present_mode: PresentMode::AutoNoVsync,
                resolution: WindowResolution::new(1920.0, 1080.0).with_scale_factor_override(1.0),
                ..default()
            }),
            ..default()
        }),
        FrameTimeDiagnosticsPlugin,
    ))
    .insert_resource(WinitSettings {
        focused_mode: UpdateMode::Continuous,
        unfocused_mode: UpdateMode::Continuous,
    })
    .insert_resource(Config {
        line_count: 50_000,
        fancy: false,
    })
    .add_systems(Startup, setup)
    .add_systems(Update, (input, ui_system));

    for _ in 0..SYSTEM_COUNT {
        app.add_systems(Update, system);
    }

    app.run();
}

#[derive(Resource, Debug)]
struct Config {
    line_count: u32,
    fancy: bool,
}

fn input(mut config: ResMut<Config>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::ArrowUp) {
        config.line_count += 10_000;
    }
    if input.just_pressed(KeyCode::ArrowDown) {
        config.line_count = config.line_count.saturating_sub(10_000);
    }
    if input.just_pressed(KeyCode::Space) {
        config.fancy = !config.fancy;
    }
}

fn system(config: Res<Config>, time: Res<Time>, mut draw: Gizmos) {
    if !config.fancy {
        for _ in 0..(config.line_count / SYSTEM_COUNT) {
            draw.line(Vec3::NEG_Y, Vec3::Y, Color::BLACK);
        }
    } else {
        for i in 0..(config.line_count / SYSTEM_COUNT) {
            let angle = i as f32 / (config.line_count / SYSTEM_COUNT) as f32 * TAU;

            let vector = Vec2::from(angle.sin_cos()).extend(time.elapsed_seconds().sin());
            let start_color = LinearRgba::rgb(vector.x, vector.z, 0.5);
            let end_color = LinearRgba::rgb(-vector.z, -vector.y, 0.5);

            draw.line_gradient(vector, -vector, start_color, end_color);
        }
    }
}

fn setup(mut commands: Commands) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3., 1., 5.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn(
        TextBundle::from_section("", TextStyle::default()).with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

fn ui_system(mut query: Query<&mut Text>, config: Res<Config>, diag: Res<DiagnosticsStore>) {
    let mut text = query.single_mut();

    let Some(fps) = diag
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
    else {
        return;
    };

    text.sections[0].value = format!(
        "Line count: {}\n\
        FPS: {:.0}\n\n\
        Controls:\n\
        Up/Down: Raise or lower the line count.\n\
        Spacebar: Toggle fancy mode.",
        config.line_count, fps,
    );
}
