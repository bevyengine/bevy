use bevy::prelude::*;
use bevy::render::camera::OrthographicProjection;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(move_sprite)
        .add_system(print_if_in_view)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_handle.into()),
        transform: Transform::from_scale(Vec3::new(20.0, 20.0, 1.0)),
        ..Default::default()
    });
}

fn move_sprite(key_state: Res<Input<KeyCode>>, mut query: Query<&mut Transform, With<Sprite>>) {
    const SPEED: f32 = 5.0;
    for mut transform in query.iter_mut() {
        if key_state.pressed(KeyCode::W) {
            let dir = transform.up();
            transform.translation += dir * SPEED;
        }
        if key_state.pressed(KeyCode::A) {
            let dir = transform.left();
            transform.translation += dir * SPEED;
        }
        if key_state.pressed(KeyCode::S) {
            let dir = transform.down();
            transform.translation += dir * SPEED;
        }
        if key_state.pressed(KeyCode::D) {
            let dir = transform.right();
            transform.translation += dir * SPEED;
        }
    }
}

fn print_if_in_view(
    camera_query: Query<(&OrthographicProjection, &GlobalTransform)>,
    sprite_query: Query<(&Transform, &Sprite), Changed<Transform>>,
) {
    for (projection, camera_transform) in camera_query.iter() {
        for (sprite_transform, sprite) in sprite_query.iter() {
            let in_view = projection.rect_in_frustum(
                camera_transform,
                Rect {
                    top: sprite_transform.translation.y + sprite.size.y,
                    left: sprite_transform.translation.x - sprite.size.x,
                    bottom: sprite_transform.translation.y - sprite.size.y,
                    right: sprite_transform.translation.x + sprite.size.x,
                },
            );
            info!(?in_view);
        }
    }
}
