use std::f32::consts::TAU;

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin},
    prelude::*,
    window::PresentMode,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            window: WindowDescriptor {
                title: "Many Debug Lines".to_string(),
                present_mode: PresentMode::AutoNoVsync,
                ..default()
            },
            ..default()
        }))
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(Config {
            line_count: 50_000,
            fancy: false,
        })
        .insert_resource(DebugDrawConfig {
            always_on_top: false,
            ..default()
        })
        .add_startup_system(setup)
        .add_system(system)
        .add_system(ui_system)
        .run();
}

#[derive(Resource, Debug)]
struct Config {
    line_count: u32,
    fancy: bool,
}

fn system(
    mut draw: ResMut<DebugDraw>,
    mut config: ResMut<Config>,
    input: Res<Input<KeyCode>>,
    time: Res<Time>,
) {
    if input.just_pressed(KeyCode::Up) {
        config.line_count += 10_000;
    }
    if input.just_pressed(KeyCode::Down) {
        config.line_count = config.line_count.saturating_sub(10_000);
    }
    if input.just_pressed(KeyCode::Space) {
        config.fancy = !config.fancy;
    }

    if !config.fancy {
        for _ in 0..config.line_count {
            draw.line(Vec3::NEG_Y, Vec3::Y, Color::BLACK);
        }
    } else {
        for i in 0..config.line_count {
            let angle = i as f32 / config.line_count as f32 * TAU;

            let vector = (Vec2::from(angle.sin_cos())).extend(time.elapsed_seconds().sin());
            let start_color = Color::rgb(vector.x, vector.z, 0.5);
            let end_color = Color::rgb(-vector.z, -vector.y, 0.5);

            draw.line_gradient(vector, -vector, start_color, end_color);
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    warn!(include_str!("warning_string.txt"));

    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(3., 1., 5.).looking_at(Vec3::ZERO, Vec3::Y),
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
