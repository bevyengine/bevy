//! Renders a lot of sprites to allow performance testing.
//! See <https://github.com/bevyengine/bevy/pull/1492>
//!
//! This example sets up many sprites in different sizes, rotations, and scales in the world.
//! It also moves the camera over them to see how well frustum culling works.
//!
//! Add the `--colored` arg to run with color tinted sprites. This will cause the sprites to be rendered
//! in multiple batches, reducing performance but useful for testing.

use bevy::{
    color::palettes::css::*,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};

use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

const COLORS: [Color; 3] = [Color::Srgba(BLUE), Color::Srgba(WHITE), Color::Srgba(RED)];

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
            FrameTimeDiagnosticsPlugin::default(),
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    ..default()
                }),
                ..default()
            }),
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (print_sprite_count, move_camera.after(print_sprite_count)),
        )
        .run();
}

fn setup(mut commands: Commands, assets: Res<AssetServer>, color_tint: Res<ColorTint>) {
    warn!(include_str!("warning_string.txt"));

    let mut rng = rand::rng();

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(320.0);

    let half_x = (map_size.x / 2.0) as i32;
    let half_y = (map_size.y / 2.0) as i32;

    let sprite_handle = assets.load("branding/icon.png");

    // Spawns the camera

    commands.spawn(Camera2d);

    // Builds and spawns the sprites
    let mut sprites = vec![];
    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.random::<f32>());
            let rotation = Quat::from_rotation_z(rng.random::<f32>());
            let scale = Vec3::splat(rng.random::<f32>() * 2.0);

            sprites.push((
                Sprite {
                    image: sprite_handle.clone(),
                    custom_size: Some(tile_size),
                    color: if color_tint.0 {
                        COLORS[rng.random_range(0..3)]
                    } else {
                        Color::WHITE
                    },
                    ..default()
                },
                Transform {
                    translation,
                    rotation,
                    scale,
                },
            ));
        }
    }
    commands.spawn_batch(sprites);
}

// System for rotating and translating the camera
fn move_camera(time: Res<Time>, mut camera_transform: Single<&mut Transform, With<Camera>>) {
    camera_transform.rotate_z(time.delta_secs() * 0.5);
    **camera_transform = **camera_transform
        * Transform::from_translation(Vec3::X * CAMERA_SPEED * time.delta_secs());
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
        info!("Sprites: {}", sprites.iter().count());
    }
}
