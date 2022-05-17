use bevy::{
    core::{Time, Timer},
    ecs::prelude::*,
    math::Vec3,
    prelude::{App, AssetServer, Assets, Handle, Transform},
    render2::{camera::OrthographicCameraBundle, color::Color, texture::Image},
    sprite2::{PipelinedSpriteBundle, Sprite},
    PipelinedDefaultPlugins,
};

struct BevyLogoLight {
    handle: Handle<Image>,
}

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

    commands.insert_resource(BevyLogoLight {
        handle: texture_handle_2,
    });
}

fn change_texture(
    time: Res<Time>,
    bevy_logo_light: Res<BevyLogoLight>,
    mut query: Query<(&mut Timer, &mut Handle<Image>)>,
) {
    for (mut timer, mut handle) in &mut query.iter_mut() {
        timer.tick(time.delta());
        if timer.finished() {
            *handle = bevy_logo_light.handle.clone();
        }
    }
}

fn change_color(
    time: Res<Time>,
    mut sprite_assets: Res<Assets<Sprite>>,
    mut query: Query<(&mut Timer, &mut Handle<Image>)>,
) {
    let (timer, sprite) = query.iter_mut().next().unwrap();
    if timer.finished() {
        sprite_assets.get_mut(sprite).unwrap().color = Color::WHITE;
    }

    // for (mut timer, mut handle) in &mut query.iter_mut() {
    //     timer.tick(time.delta());
    // }
}
