use bevy::{
    core::FixedTimestep,
    diagnostic::{Diagnostics, FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};
use rand::random;

const BIRDS_PER_SECOND: u32 = 10000;
const _BASE_COLOR: Color = Color::rgb(5.0, 5.0, 5.0);
const GRAVITY: f32 = -9.8 * 100.0;
const MAX_VELOCITY: f32 = 750.;
const BIRD_SCALE: f32 = 0.15;
const HALF_BIRD_SIZE: f32 = 256. * BIRD_SCALE * 0.5;

struct BevyCounter {
    pub count: u128,
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
            vsync: false,
            resizable: true,
            ..Default::default()
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
    wave: u128,
    per_wave: u128,
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
            bird_texture.0.clone_weak(),
        );
        counter.color = Color::rgb_linear(random(), random(), random());
        scheduled.wave -= 1;
    }
}

struct BirdTexture(Handle<Image>);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture = asset_server.load("branding/icon.png");

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(UiCameraBundle::default());
    commands.spawn_bundle(TextBundle {
        text: Text {
            sections: vec![
                TextSection {
                    value: "Bird Count: ".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 0.0),
                    },
                },
                TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 1.0),
                    },
                },
                TextSection {
                    value: "\nAverage FPS: ".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 0.0),
                    },
                },
                TextSection {
                    value: "".to_string(),
                    style: TextStyle {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        font_size: 40.0,
                        color: Color::rgb(0.0, 1.0, 1.0),
                    },
                },
            ],
            ..Default::default()
        },
        style: Style {
            position_type: PositionType::Absolute,
            position: Rect {
                top: Val::Px(5.0),
                left: Val::Px(5.0),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    });

    commands.insert_resource(BirdTexture(texture));
    commands.insert_resource(BirdScheduled {
        per_wave: std::env::args()
            .nth(1)
            .and_then(|arg| arg.parse::<u128>().ok())
            .unwrap_or_default(),
        wave: std::env::args()
            .nth(2)
            .and_then(|arg| arg.parse::<u128>().ok())
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
        counter.color = Color::rgb_linear(random(), random(), random());
    }

    if mouse_button_input.pressed(MouseButton::Left) {
        let spawn_count = (BIRDS_PER_SECOND as f64 * time.delta_seconds_f64()) as u128;
        spawn_birds(
            &mut commands,
            &windows,
            &mut counter,
            spawn_count,
            bird_texture.0.clone_weak(),
        );
    }
}

fn spawn_birds(
    commands: &mut Commands,
    windows: &Windows,
    counter: &mut BevyCounter,
    spawn_count: u128,
    texture: Handle<Image>,
) {
    let window = windows.get_primary().unwrap();
    let bird_x = (window.width() as f32 / -2.) + HALF_BIRD_SIZE;
    let bird_y = (window.height() as f32 / 2.) - HALF_BIRD_SIZE;
    for count in 0..spawn_count {
        let bird_z = (counter.count + count) as f32 * 0.00001;
        commands
            .spawn_bundle(SpriteBundle {
                texture: texture.clone(),
                transform: Transform {
                    translation: Vec3::new(bird_x, bird_y, bird_z),
                    scale: Vec3::splat(BIRD_SCALE),
                    ..Default::default()
                },
                sprite: Sprite {
                    color: counter.color,
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert(Bird {
                velocity: Vec3::new(
                    rand::random::<f32>() * MAX_VELOCITY - (MAX_VELOCITY * 0.5),
                    0.,
                    0.,
                ),
            });
    }
    counter.count += spawn_count;
}

fn movement_system(time: Res<Time>, mut bird_query: Query<(&mut Bird, &mut Transform)>) {
    for (mut bird, mut transform) in bird_query.iter_mut() {
        transform.translation.x += bird.velocity.x * time.delta_seconds();
        transform.translation.y += bird.velocity.y * time.delta_seconds();
        bird.velocity.y += GRAVITY * time.delta_seconds();
    }
}

fn collision_system(windows: Res<Windows>, mut bird_query: Query<(&mut Bird, &Transform)>) {
    let window = windows.get_primary().unwrap();
    let half_width = window.width() as f32 * 0.5;
    let half_height = window.height() as f32 * 0.5;

    for (mut bird, transform) in bird_query.iter_mut() {
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
    mut query: Query<&mut Text>,
) {
    if let Some(fps) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
        if let Some(average) = fps.average() {
            for mut text in query.iter_mut() {
                text.sections[1].value = format!("{}", counter.count);
                text.sections[3].value = format!("{:.2}", average);
            }
        }
    };
}
