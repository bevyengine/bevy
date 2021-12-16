use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands, mut materials: ResMut<Assets<ColorMaterial>>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        transform: Transform {
            scale: Vec3::new(50.0, 50.0, 0.0),
            ..Default::default()
        },
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            ..Default::default()
        },
    });
}
