use bevy::{
    core::{Time, Timer},
    ecs::prelude::*,
    math::Vec3,
    prelude::{App, AssetServer, Handle, Transform},
    render2::{camera::OrthographicCameraBundle, color::Color, texture::Image},
    sprite2::{PipelinedSpriteBundle, Sprite},
    PipelinedDefaultPlugins,
};

fn main() {
    App::new()
        .add_plugins(PipelinedDefaultPlugins)
        .add_startup_system(setup)
        .add_system(change_texture)
        .add_system(change_color)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture_handle = asset_server.load("branding/bevy_logo_dark.png");
    let texture_handle_2: Handle<Image> = asset_server.load("branding/bevy_logo_light.png");
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(PipelinedSpriteBundle {
            texture: texture_handle.clone(),
            transform: Transform {
                translation: Vec3::new(1., 1., 1.),
                scale: Vec3::ONE,
                ..Default::default()
            },
            sprite: Sprite {
                color: Color::WHITE,
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(Timer::from_seconds(1.5, false));

    commands.insert_resource(texture_handle_2);
}

fn change_texture(
    time: Res<Time>,
    texture: Res<Handle<Image>>,
    mut query: Query<(&mut Timer, &mut Handle<Image>)>,
) {
    for (mut timer, mut handle) in &mut query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            *handle = texture.clone();
        }
    }
}

fn change_color(time: Res<Time>, mut query: Query<(&mut Timer, &mut Sprite)>) {
    for (mut timer, mut sprite) in &mut query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            sprite.color = Color::rgb(1.0, 0.0, 0.0);
        }
    }
}
