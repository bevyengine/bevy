use bevy::{prelude::*, render::camera::Camera};

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(follow.system())
        .run();
}

struct Follow;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        })
        .insert(Follow);
}

fn follow(
    mut q: Query<&mut Transform, With<Follow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    windows: Res<Windows>,
    mut evr_cursor: EventReader<CursorMoved>,
) {
    if let Ok((camera, camera_transform)) = q_camera.single() {
        if let Some(cursor) = evr_cursor.iter().next() {
            for mut transform in q.iter_mut() {
                let point: Option<Vec3> =
                    Camera::screen_to_point_2d(cursor.position, &windows, camera, camera_transform);
                if let Some(point) = point {
                    transform.translation = point;
                }
            }
        }
    }
}
