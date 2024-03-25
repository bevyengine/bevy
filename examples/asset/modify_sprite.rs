//! Demonstrates changing sprite texture and color at runtime.
//!
//! Loads two different [`Image`]s on startup, then uses the first one as the texture for a sprite.
//! After some time, the sprite's texture is replaced with the second `Image`, and its color is modified.
use bevy::prelude::*;

struct BevyLogoLight {
    handle: Handle<Image>,
}

#[derive(Component, Deref, DerefMut)]
struct SpriteTimer(Timer);

#[derive(Component, Deref, DerefMut)]
struct ColorTimer(Timer);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(change_texture)
        .add_system(change_color)
        .run();
}

/// Sets up the scene, creating the sprite and loading the textures.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Load our textures
    let first_texture = asset_server.load("branding/bevy_logo_dark.png");
    let texture_to_set_after_time: Handle<Image> =
        asset_server.load("branding/bevy_logo_light.png");
    // Setup our Sprite with the first texture
    commands
        .spawn_bundle(SpriteBundle {
            texture: first_texture,
            ..default()
        })
        .insert(SpriteTimer(Timer::from_seconds(1f32, false)))
        .insert(ColorTimer(Timer::from_seconds(2f32, false)));

    // Our texture that we want to apply to our SpriteBundle at runtime
    commands.insert_resource(BevyLogoLight {
        handle: texture_to_set_after_time,
    });

    commands.spawn_bundle(Camera2dBundle::default());
}

/// Changes the sprite texture by using the image handle when `AnimationTimer` finishes.
fn change_texture(
    time: Res<Time>,
    bevy_logo_light: Res<BevyLogoLight>,
    mut query: Query<(&mut SpriteTimer, &mut Handle<Image>)>,
) {
    for (mut timer, mut handle) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            *handle = bevy_logo_light.handle.clone();
        }
    }
}

/// Changes the sprite color by mutating the sprite asset when `AnimationTimer` finishes.
fn change_color(time: Res<Time>, mut query: Query<(&mut ColorTimer, &mut Sprite)>) {
    for (mut timer, mut sprite) in query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            sprite.color = Color::RED;
        }
    }
}
