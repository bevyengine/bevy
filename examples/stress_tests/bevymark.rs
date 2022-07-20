//! This example provides a 2D benchmark.
//!
//! Usage: spawn more entities by clicking on the screen.

use bevy::{
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    time::FixedTimestep,
    window::PresentMode,
};
use rand::{thread_rng, Rng};

const BIRDS_PER_SECOND: u32 = 10000;
const GRAVITY: f32 = -9.8 * 100.0;
const MAX_VELOCITY: f32 = 750.;
const BIRD_SCALE: f32 = 0.15;
const HALF_BIRD_SIZE: f32 = 256. * BIRD_SCALE * 0.5;

struct BevyCounter {
    pub count: usize,
    pub color: Color,
}

#[derive(Component)]
struct Bird {
    velocity: Vec3,
}

fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            title: "BevyMark".to_string(),
            width: 800.,
            height: 600.,
            present_mode: PresentMode::AutoNoVsync,
            resizable: true,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .insert_resource(BevyCounter {
            count: 0,
            color: Color::WHITE,
        })
        .add_startup_system(setup)
        .add_system(mouse_handler)
        .add_system(movement_system)
        .add_system(collision_system)
        .add_system(counter_system)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.2))
                .with_system(scheduled_spawner),
        )
        .run();
}

struct BirdScheduled {
    wave: usize,
    per_wave: usize,
}

fn scheduled_spawner(
    mut commands: Commands,
    windows: Res<Windows>,
    mut scheduled: ResMut<BirdScheduled>,
    mut counter: ResMut<BevyCounter>,
    bird_texture: Res<BirdTexture>,
) {
    if scheduled.wave > 0 {
        spawn_birds(
            &mut commands,
            &windows,
            &mut counter,
            scheduled.per_wave,
            bird_texture.clone_weak(),
        );

        let mut rng = thread_rng();
        counter.color = Color::rgb_linear(rng.gen(), rng.gen(), rng.gen());
        scheduled.wave -= 1;
    }
}

#[derive(Deref)]
struct BirdTexture(Handle<Image>);

#[derive(Component)]
struct StatsText;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    warn!(include_str!("warning_string.txt"));

    let texture = asset_server.load("branding/icon.png");

    commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn_bundle(
            TextBundle::from_sections([
                TextSection::new(
                    "Bird Count: ",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 0.0),
                    },
                ),
                TextSection::from_style(TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::rgb(0.0, 1.0, 1.0),
                }),
                TextSection::new(
                    "\nAverage FPS: ",
                    TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 0.0),
                    },
                ),
                TextSection::from_style(TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 40.0,
                    color: Color::rgb(0.0, 1.0, 1.0),
                }),
            ])
            .with_style(Style {
                position_type: PositionType::Absolute,
                position: UiRect {
                    top: Val::Px(5.0),
                    left: Val::Px(5.0),
                    ..default()
                },
                ..default()
            }),
        )
        .insert(StatsText);

    commands.insert_resource(BirdTexture(texture));
    commands.insert_resource(BirdScheduled {
        per_wave: std::env::args()
            .nth(1)
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or_default(),
        wave: std::env::args()
            .nth(2)
            .and_then(|arg| arg.parse::<usize>().ok())
            .unwrap_or(1),
    });
}

fn mouse_handler(
    mut commands: Commands,
    time: Res<Time>,
    mouse_button_input: Res<Input<MouseButton>>,
    windows: Res<Windows>,
    bird_texture: Res<BirdTexture>,
    mut counter: ResMut<BevyCounter>,
) {
    if mouse_button_input.just_released(MouseButton::Left) {
        let mut rng = thread_rng();
        counter.color = Color::rgb_linear(rng.gen(), rng.gen(), rng.gen());
    }

    if mouse_button_input.pressed(MouseButton::Left) {
        let spawn_count = (BIRDS_PER_SECOND as f64 * time.delta_seconds_f64()) as usize;
        spawn_birds(
            &mut commands,
            &windows,
            &mut counter,
            spawn_count,
            bird_texture.clone_weak(),
        );
    }
}

fn spawn_birds(
    commands: &mut Commands,
    windows: &Windows,
    counter: &mut BevyCounter,
    spawn_count: usize,
    texture: Handle<Image>,
) {
    let window = windows.primary();
    let bird_x = (window.width() as f32 / -2.) + HALF_BIRD_SIZE;
    let bird_y = (window.height() as f32 / 2.) - HALF_BIRD_SIZE;
    let mut rng = thread_rng();

    for count in 0..spawn_count {
        let bird_z = (counter.count + count) as f32 * 0.00001;
        commands
            .spawn_bundle(SpriteBundle {
                texture: texture.clone(),
                transform: Transform {
                    translation: Vec3::new(bird_x, bird_y, bird_z),
                    scale: Vec3::splat(BIRD_SCALE),
                    ..default()
                },
                sprite: Sprite {
                    color: counter.color,
                    ..default()
                },
                ..default()
            })
            .insert(Bird {
                velocity: Vec3::new(
                    rng.gen::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
                    0.,
                    0.,
                ),
            });
    }
    counter.count += spawn_count;
}

fn movement_system(time: Res<Time>, mut bird_query: Query<(&mut Bird, &mut Transform)>) {
    for (mut bird, mut transform) in &mut bird_query {
        transform.translation.x += bird.velocity.x * time.delta_seconds();
        transform.translation.y += bird.velocity.y * time.delta_seconds();
        bird.velocity.y += GRAVITY * time.delta_seconds();
    }
}

fn collision_system(windows: Res<Windows>, mut bird_query: Query<(&mut Bird, &Transform)>) {
    let window = windows.primary();
    let half_width = window.width() as f32 * 0.5;
    let half_height = window.height() as f32 * 0.5;

    for (mut bird, transform) in &mut bird_query {
        let x_vel = bird.velocity.x;
        let y_vel = bird.velocity.y;
        let x_pos = transform.translation.x;
        let y_pos = transform.translation.y;

        if (x_vel > 0. && x_pos + HALF_BIRD_SIZE > half_width)
            || (x_vel <= 0. && x_pos - HALF_BIRD_SIZE < -(half_width))
        {
            bird.velocity.x = -x_vel;
        }
        if y_vel < 0. && y_pos - HALF_BIRD_SIZE < -half_height {
            bird.velocity.y = -y_vel;
        }
        if y_pos + HALF_BIRD_SIZE > half_height && y_vel > 0.0 {
            bird.velocity.y = 0.0;
        }
    }
}

fn counter_system(
    diagnostics: Res<Diagnostics>,
    counter: Res<BevyCounter>,
    mut query: Query<&mut Text, With<StatsText>>,
) {
    let mut text = query.single_mut();

    if counter.is_changed() {
        text.sections[1].value = counter.count.to_string();
    }

    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            text.sections[3].value = format!("{average:.2}");
        }
    };
}
