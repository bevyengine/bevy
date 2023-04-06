use std::cmp::max;
use std::f32::consts::TAU;

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};

const SYSTEM_COUNT: u32 = 10;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Many Debug Lines".to_string(),
            present_mode: PresentMode::AutoNoVsync,
            ..default()
        }),
        ..default()
    }))
    .add_plugin(FrameTimeDiagnosticsPlugin::default())
    .insert_resource(Config {
        line_count: 50_000,
        fancy: false,
    })
    .insert_resource(GizmoConfig {
        on_top: false,
        ..default()
    })
    .add_systems(Startup, setup)
    .add_systems(Update, (input, ui_system));

    let make_system = |index: u32| {
        move |config: Res<Config>, time: Res<Time>, draw: Gizmos| system(index, config, time, draw)
    };

    for index in 0..SYSTEM_COUNT {
        app.add_systems(Update, make_system(index));
    }

    app.run();
}

#[derive(Resource, Debug)]
struct Config {
    line_count: u32,
    fancy: bool,
}

fn input(mut config: ResMut<Config>, input: Res<Input<KeyCode>>) {
    if input.just_pressed(KeyCode::Up) {
        config.line_count = max((config.line_count as f32 * 1.2).ceil() as u32, 1);
    }
    if input.just_pressed(KeyCode::Down) {
        config.line_count = (config.line_count as f32 / 1.2).floor() as u32;
    }
    if input.just_pressed(KeyCode::Space) {
        config.fancy = !config.fancy;
    }
}

fn system(index: u32, config: Res<Config>, time: Res<Time>, mut draw: Gizmos) {
    let mut rand_state = (time.elapsed_seconds() as i32 as f32 * (index + 1) as f32) as i32;

    let mut rand = || -> f32 {
        rand_state = rand_state.wrapping_mul(1103515245).wrapping_add(12345) & 0x7fffffff;
        (rand_state as f32 / 0x3fffffff as f32) - 1.
    };

    let line_count =
        config.line_count / SYSTEM_COUNT + (index < config.line_count % SYSTEM_COUNT) as u32;

    draw.cuboid(
        Vec3::ZERO,
        Quat::IDENTITY,
        Vec3::new(2., 2., 2.),
        Color::BLACK,
    );

    for i in 0..line_count {
        let start = Vec3::new(rand(), rand(), rand());
        let end = Vec3::new(rand(), rand(), rand());

        if !config.fancy {
            draw.line(start, end, Color::WHITE);
        } else {
            let angle = (i * SYSTEM_COUNT + index) as f32 / config.line_count as f32 * TAU;
            let vector = Vec2::from(angle.sin_cos()).extend(time.elapsed_seconds().sin());
            let start_color = Color::rgb(vector.x, vector.z, 0.5);
            let end_color = Color::rgb(-vector.z, -vector.y, 0.5);
            draw.line_gradient(start, end, start_color, end_color);
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3., 2., 4.).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    commands.spawn(TextBundle::from_section(
        "",
        TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 30.,
            ..default()
        },
    ));
}

fn ui_system(mut query: Query<&mut Text>, config: Res<Config>, diag: Res<Diagnostics>) {
    let mut text = query.single_mut();

    let Some(fps) = diag.get(FrameTimeDiagnosticsPlugin::FPS).and_then(|fps| fps.smoothed()) else {
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
