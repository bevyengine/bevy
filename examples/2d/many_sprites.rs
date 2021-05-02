use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::Quat,
    prelude::*,
    render::camera::Camera,
    sprite::SpriteSettings,
};

use rand::Rng;

const CAMERA_SPEED: f32 = 1.0;

pub struct PrintTimer(Timer);

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
        .add_startup_system(setup.system())
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
        .insert_bundle(OrthographicCameraBundle {
            transform: Transform::from_xyz(map_size.x * tile_size.x / 4.0, 0.0, 1000.0),
            ..OrthographicCameraBundle::new_2d()
        })
        .insert(PrintTimer(Timer::from_seconds(1.0, true)));

    let mut sprites = Vec::with_capacity((map_size.x * map_size.y) as usize);
    for y in -half_y..half_y {
        for x in -half_x..half_x {
            let position = Vec2::new(x as f32, y as f32);
            let translation = (position * tile_size).extend(rng.gen::<f32>());
            let rotation = Quat::from_rotation_z(rng.gen::<f32>());
            let scale = Vec3::splat(rng.gen::<f32>() * 2.0);

            sprites.push(SpriteBundle {
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
    commands.spawn_batch(sprites);
}

fn move_camera(time: Res<Time>, mut query: Query<&mut Transform, With<Camera>>) {
    if let Ok(mut transform) = query.single_mut() {
        *transform =
            Transform::from_rotation(Quat::from_rotation_z(CAMERA_SPEED * time.delta_seconds()))
                * Transform::from_translation(transform.translation);
        transform.rotate(Quat::from_rotation_z(
            CAMERA_SPEED * time.seconds_since_startup() as f32,
        ));
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
