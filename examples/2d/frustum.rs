use bevy::{
    prelude::*,
    render::draw::OutsideFrustum,
    sprite::SpriteSettings,
};
use bevy::render::camera::OrthographicProjection;

struct Bar;
struct PrintTimer(Timer);

fn setup(
    mut commands: Commands,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    let mut transform = Transform::from_xyz(-400.0, 0.0, 0.0);
    transform.scale = Vec3::new(1.0, 20.0, 1.0);

    commands.spawn_bundle(SpriteBundle {
        material: materials.add(Color::rgb(0.5, 0.5, 1.0).into()),
        transform,
        sprite: Sprite::new(Vec2::new(30.0, 30.0)),
        ..Default::default()
    }).insert(Bar);
}

fn rotate(mut query: Query<&mut Transform, With<Bar>>, time: Res<Time>) {
    let speed = 5f32;
    for mut t in query.iter_mut() {
        t.rotation *= Quat::from_rotation_z(time.delta_seconds() * speed);
    }
}

fn travel_camera(keys: Res<Input<KeyCode>>, mut query: Query<&mut Transform, With<OrthographicProjection>>, time: Res<Time>) {
    let speed = 2f32;
    for mut t in query.iter_mut() {
        if keys.pressed(KeyCode::S) {
            t.scale += time.delta_seconds() * speed;
        }
        if keys.pressed(KeyCode::W) {
            t.scale -= time.delta_seconds() * speed;
        }
    }
}

fn travel(keys: Res<Input<KeyCode>>, mut query: Query<&mut Transform, With<Bar>>, time: Res<Time>) {
    let speed = 500f32;
    for mut t in query.iter_mut() {
        if keys.pressed(KeyCode::Right) {
            t.translation.x += time.delta_seconds() * speed;
        }
        if keys.pressed(KeyCode::Left) {
            t.translation.x -= time.delta_seconds() * speed;
        }
        if keys.pressed(KeyCode::Up) {
            t.translation.y += time.delta_seconds() * speed;
        }
        if keys.pressed(KeyCode::Down) {
            t.translation.y -= time.delta_seconds() * speed;
        }
    }
}

fn info(time: Res<Time>, mut timer: ResMut<PrintTimer>, query: Query<&Bar, Without<OutsideFrustum>>) {
    let mut count = 0;
    for _ in query.iter() {
        count += 1;
    }
    if timer.0.tick(time.delta()).just_finished() {
        info!("{} sprites on screen", count);
    }
}

fn startup() {
    info!("use the arrow keys to move the bar, 'W' & 'S' to scale the camera");
}

fn main() {
    App::new()
        .insert_resource(SpriteSettings {
            frustum_culling_enabled: true,
        })
        .insert_resource(PrintTimer(Timer::from_seconds(1.0, true)))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_startup_system(startup.system())
        .add_system(rotate.system())
        .add_system(travel.system())
        .add_system(travel_camera.system())
        .add_system(info.system())
        .run();
}
