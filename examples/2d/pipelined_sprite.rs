use bevy::math::Vec3;
use bevy::prelude::{App, AssetServer, Commands, Res, Transform};
use bevy::render2::camera::OrthographicCameraBundle;
use bevy::sprite2::{PipelinedSpriteBundle, Sprite};
use bevy::PipelinedDefaultPlugins;

fn main() {
    App::new()
        .add_plugins(PipelinedDefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture_handle = asset_server.load("branding/banner.png");
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(PipelinedSpriteBundle {
        sprite: Sprite {
            border_radius: 14.0,
            ..Default::default()
        },
        texture: texture_handle,
        transform: Transform::from_scale(Vec3::new(0.5, 0.5, 1.0)),
        ..Default::default()
    });
}
