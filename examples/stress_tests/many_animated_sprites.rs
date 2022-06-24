//! Renders a lot of animated sprites to allow performance testing.
//!
//! It sets up many animated sprites in different sizes and rotations, and at different scales in the world,
//! and moves the camera over them to see how well frustum culling works.
//!
//! To measure performance realistically, be sure to run this in release mode.
//! `cargo run --example many_animated_sprites --release`

use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Quat,
    prelude::*,
    render::camera::Camera,
};

use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

fn main() {
    App::new()
        // Since this is also used as a benchmark, we want it to display performance data.
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(animate_sprite)
        .add_system(print_sprite_count)
        .add_system(move_camera.after(print_sprite_count))
        .run();
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let mut rng = rand::thread_rng();

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(320.0);

    let half_x = (map_size.x / 2.0) as i32;
    let half_y = (map_size.y / 2.0) as i32;

    let texture_handle = assets.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // Spawns the camera
    commands
        .spawn()
        .insert_bundle(Camera2dBundle::default())
        .insert(Transform::from_xyz(0.0, 0.0, 1000.0));

    // Builds and spawns the sprites
    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.gen::<f32>());
            let rotation = Quat::from_rotation_z(rng.gen::<f32>());
            let scale = Vec3::splat(rng.gen::<f32>() * 2.0);
            let mut timer = Timer::from_seconds(0.1, true);
            timer.set_elapsed(Duration::from_secs_f32(rng.gen::<f32>()));

            commands
                .spawn_bundle(SpriteSheetBundle {
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
                })
                .insert(AnimationTimer(timer));
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
        Self(Timer::from_seconds(1.0, true))
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
        info!("Sprites: {}", sprites.iter().count(),);
    }
}
