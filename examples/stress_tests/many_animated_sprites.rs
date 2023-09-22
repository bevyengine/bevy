//! Renders a lot of animated sprites to allow performance testing.
//!
//! This example sets up many animated sprites in different sizes, rotations, and scales in the world.
//! It also moves the camera over them to see how well frustum culling works.

use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Quat,
    prelude::*,
    render::camera::Camera,
    window::PresentMode,
};

use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

fn main() {
    App::new()
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
            (
                animate_sprite,
                print_sprite_count,
                move_camera.after(print_sprite_count),
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    warn!(include_str!("warning_string.txt"));

    let mut rng = rand::thread_rng();

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(320.0);

    let half_x = (map_size.x / 2.0) as i32;
    let half_y = (map_size.y / 2.0) as i32;

    let texture_handle = assets.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let texture_atlas =
        TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // Spawns the camera

    commands.spawn(Camera2dBundle::default());

    // Builds and spawns the sprites
    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.gen::<f32>());
            let rotation = Quat::from_rotation_z(rng.gen::<f32>());
            let scale = Vec3::splat(rng.gen::<f32>() * 2.0);
            let mut timer = Timer::from_seconds(0.1, TimerMode::Repeating);
            timer.set_elapsed(Duration::from_secs_f32(rng.gen::<f32>()));

            commands.spawn((
                SpriteSheetBundle {
                    texture_atlas: texture_atlas_handle.clone(),
                    transform: Transform {
                        translation,
                        rotation,
                        scale,
                    },
                    sprite: TextureAtlasSprite {
                        custom_size: Some(tile_size),
                        ..default()
                    },
                    ..default()
                },
                AnimationTimer(timer),
            ));
        }
    }
}

// System for rotating and translating the camera
fn move_camera(time: Res<Time>, mut camera_query: Query<&mut Transform, With<Camera>>) {
    let mut camera_transform = camera_query.single_mut();
    camera_transform.rotate(Quat::from_rotation_z(time.delta_seconds() * 0.5));
    *camera_transform = *camera_transform
        * Transform::from_translation(Vec3::X * CAMERA_SPEED * time.delta_seconds());
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(
        &mut AnimationTimer,
        &mut TextureAtlasSprite,
        &Handle<TextureAtlas>,
    )>,
) {
    for (mut timer, mut sprite, texture_atlas_handle) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            let texture_atlas = texture_atlases.get(texture_atlas_handle).unwrap();
            sprite.index = (sprite.index + 1) % texture_atlas.textures.len();
        }
    }
}

#[derive(Deref, DerefMut)]
struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}

// System for printing the number of sprites on every tick of the timer
fn print_sprite_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    sprites: Query<&TextureAtlasSprite>,
) {
    timer.tick(time.delta());

    if timer.just_finished() {
        info!("Sprites: {}", sprites.iter().count());
    }
}
