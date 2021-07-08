use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Quat,
    prelude::*,
    sprite::SpriteSettings,
};

use rand::Rng;

const CAMERA_SPEED: f32 = 1000.0;

pub struct PrintTimer(Timer);
pub struct Position(Transform);

/// This example is for performance testing purposes.
/// See https://github.com/bevyengine/bevy/pull/1492
fn main() {
    App::build()
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .insert_resource(SpriteSettings {
            // NOTE: this is an experimental feature that doesn't work in all cases
            frustum_culling_enabled: true,
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(tick.system().label("Tick"))
        .add_system(move_camera.system().after("Tick"))
        .run()
}

fn setup(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let mut rng = rand::thread_rng();

    let tile_size = Vec2::splat(64.0);
    let map_size = Vec2::splat(320.0);

    let half_x = (map_size.x / 2.0) as i32;
    let half_y = (map_size.y / 2.0) as i32;

    let sprite_handle = materials.add(assets.load("branding/icon.png").into());

    commands
        .spawn()
        .insert_bundle(OrthographicCameraBundle::new_2d())
        .insert(PrintTimer(Timer::from_seconds(1.0, true)))
        .insert(Position(Transform::from_translation(Vec3::new(
            0.0, 0.0, 1000.0,
        ))));

    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.gen::<f32>());
            let rotation = Quat::from_rotation_z(rng.gen::<f32>());
            let scale = Vec3::splat(rng.gen::<f32>() * 2.0);

            commands.spawn().insert_bundle(SpriteBundle {
                material: sprite_handle.clone(),
                transform: Transform {
                    translation,
                    rotation,
                    scale,
                },
                sprite: Sprite::new(tile_size),
                ..Default::default()
            });
        }
    }
}

fn move_camera(time: Res<Time>, mut query: Query<(&mut Transform, &mut Position)>) {
    for (mut transform, mut position) in query.iter_mut() {
        position
            .0
            .rotate(Quat::from_rotation_z(time.delta_seconds() * 0.5));
        position.0 =
            position.0 * Transform::from_translation(Vec3::X * CAMERA_SPEED * time.delta_seconds());
        transform.translation = position.0.translation;
        transform.rotation *= Quat::from_rotation_z(time.delta_seconds() / 2.0);
    }
}

fn tick(time: Res<Time>, sprites: Query<&Sprite>, mut query: Query<&mut PrintTimer>) {
    for mut timer in query.iter_mut() {
        timer.0.tick(time.delta());

        if timer.0.just_finished() {
            info!("Sprites: {}", sprites.iter().count(),);
        }
    }
}
