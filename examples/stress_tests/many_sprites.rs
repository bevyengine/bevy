//! Renders a lot of sprites to allow performance testing.
//! See <https://github.com/bevyengine/bevy/pull/1492>
//!
//! This example sets up many sprites in different sizes, rotations, and scales in the world.
//! It also moves the camera over them to see how well frustum culling works.
//!
//! Add the `--colored` arg to run with color tinted sprites. This will cause the sprites to be rendered
//! in multiple batches, reducing performance but useful for testing.

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

const COLORS: [Color; 3] = [Color::BLUE, Color::WHITE, Color::RED];

#[derive(Resource)]
struct ColorTint(bool);

fn main() {
    App::new()
        .insert_resource(ColorTint(
            std::env::args().nth(1).unwrap_or_default() == "--colored",
        ))
        // Since this is also used as a benchmark, we want it to display performance data.
        .add_plugins((
            LogDiagnosticsPlugin::default(),
            FrameTimeDiagnosticsPlugin,
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (print_sprite_count, move_camera.after(print_sprite_count)),
        )
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>, color_tint: Res<ColorTint>) {
    warn!(include_str!("warning_string.txt"));

    let mut rng = rand::thread_rng();

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(320.0);

    let half_x = (map_size.x / 2.0) as i32;
    let half_y = (map_size.y / 2.0) as i32;

    let sprite_handle = assets.load("branding/icon.png");

    // Spawns the camera

    commands.spawn(Camera2dBundle::default());

    // Builds and spawns the sprites
    let mut sprites = vec![];
    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.gen::<f32>());
            let rotation = Quat::from_rotation_z(rng.gen::<f32>());
            let scale = Vec3::splat(rng.gen::<f32>() * 2.0);

            sprites.push(SpriteBundle {
                texture: sprite_handle.clone(),
                transform: Transform {
                    translation,
                    rotation,
                    scale,
                },
                sprite: Sprite {
                    custom_size: Some(tile_size),
                    color: if color_tint.0 {
                        COLORS[rng.gen_range(0..3)]
                    } else {
                        Color::WHITE
                    },
                    ..default()
                },
                ..default()
            });
        }
    }
    commands.spawn_batch(sprites);
}

// System for rotating and translating the camera
fn move_camera(time: Res<Time>, mut camera_query: Query<&mut Transform, With<Camera>>) {
    let mut camera_transform = camera_query.single_mut();
    camera_transform.rotate_z(time.delta_seconds() * 0.5);
    *camera_transform = *camera_transform
        * Transform::from_translation(Vec3::X * CAMERA_SPEED * time.delta_seconds());
}

#[derive(Deref, DerefMut)]
struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

// System for printing the number of sprites on every tick of the timer
fn print_sprite_count(time: Res<Time>, mut timer: Local<PrintingTimer>, sprites: Query<&Sprite>) {
    timer.tick(time.delta());

    if timer.just_finished() {
        info!("Sprites: {}", sprites.iter().count(),);
    }
}
