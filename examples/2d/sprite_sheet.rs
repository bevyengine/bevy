//! Renders an animated sprite by loading all animation frames from a single image (a sprite sheet)
//! into a texture atlas, and changing the displayed image periodically.

use bevy::{prelude::*, render::texture::ImageSettings};
use std::ops::DerefMut;

fn main() {
    App::new()
        .insert_resource(ImageSettings::default_nearest()) // prevents blurry sprites
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(animate_sprite)
        .run();
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    texture_atlases: Res<Assets<TextureAtlas>>,
    mut query: Query<(&mut AnimationTimer, &mut SpriteImage)>,
) {
    for (mut timer, mut image) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.just_finished() {
            if let SpriteImage::TextureAtlas { index, handle } = image.deref_mut() {
                let texture_atlas = texture_atlases.get(handle).unwrap();
                *index = (*index + 1) % texture_atlas.textures.len();
            }
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlases: ResMut<Assets<TextureAtlas>>,
) {
    let texture_handle = asset_server.load("textures/rpg/chars/gabe/gabe-idle-run.png");
    let texture_atlas = TextureAtlas::from_grid(texture_handle, Vec2::new(24.0, 24.0), 7, 1);
    let texture_atlas_handle = texture_atlases.add(texture_atlas);
    commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn_bundle(SpriteBundle {
            texture: SpriteImage::TextureAtlas {
                handle: texture_atlas_handle,
                index: 0,
            },
            transform: Transform::from_scale(Vec3::splat(6.0)),
            ..default()
        })
        .insert(AnimationTimer(Timer::from_seconds(0.1, true)));
}
