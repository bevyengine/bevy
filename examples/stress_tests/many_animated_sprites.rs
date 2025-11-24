//! Renders a lot of animated sprites to allow performance testing.
//!
//! This example sets up many animated sprites in different sizes, rotations, and scales in the world.
//! It also moves the camera over them to see how well frustum culling works.

use std::time::Duration;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};

use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

fn main() {
    App::new()
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
        .insert_resource(WinitSettings::continuous())
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
    mut texture_atlases: ResMut<Assets<TextureAtlasLayout>>,
) {
    warn!(include_str!("warning_string.txt"));

    let mut rng = rand::rng();

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(320.0);

    let half_x = (map_size.x / 2.0) as i32;
    let half_y = (map_size.y / 2.0) as i32;

    let texture_handle = assets.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let texture_atlas = TextureAtlasLayout::from_grid(UVec2::splat(24), 7, 1, None, None);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);

    // Spawns the camera

    commands.spawn(Camera2d);

    // Builds and spawns the sprites
    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.random::<f32>());
            let rotation = Quat::from_rotation_z(rng.random::<f32>());
            let scale = Vec3::splat(rng.random::<f32>() * 2.0);
            let mut timer = Timer::from_seconds(0.1, TimerMode::Repeating);
            timer.set_elapsed(Duration::from_secs_f32(rng.random::<f32>()));

            commands.spawn((
                Sprite {
                    image: texture_handle.clone(),
                    texture_atlas: Some(TextureAtlas::from(texture_atlas_handle.clone())),
                    custom_size: Some(tile_size),
                    ..default()
                },
                Transform {
                    translation,
                    rotation,
                    scale,
                },
                AnimationTimer(timer),
            ));
        }
    }
}

// System for rotating and translating the camera
fn move_camera(time: Res<Time>, mut camera_transform: Single<&mut Transform, With<Camera>>) {
    camera_transform.rotate(Quat::from_rotation_z(time.delta_secs() * 0.5));
    **camera_transform = **camera_transform
        * Transform::from_translation(Vec3::X * CAMERA_SPEED * time.delta_secs());
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlasLayout>>,
    mut query: Query<(&mut AnimationTimer, &mut Sprite)>,
) {
    for (mut timer, mut sprite) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            let Some(atlas) = &mut sprite.texture_atlas else {
                continue;
            };
            let texture_atlas = texture_atlases.get(&atlas.layout).unwrap();
            atlas.index = (atlas.index + 1) % texture_atlas.textures.len();
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
fn print_sprite_count(time: Res<Time>, mut timer: Local<PrintingTimer>, sprites: Query<&Sprite>) {
    timer.tick(time.delta());

    if timer.just_finished() {
        info!("Sprites: {}", sprites.iter().count());
    }
}
