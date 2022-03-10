use bevy::{prelude::*, render::camera::Camera};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(follow)
        .run();
}

#[derive(Component)]
struct Follow;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture_handle = asset_server.load("branding/icon.png");
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    commands
        .spawn_bundle(SpriteBundle {
            texture: texture_handle,
            ..Default::default()
        })
        .insert(Follow);
}

fn follow(
    mut q: Query<&mut Transform, With<Follow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    mut evr_cursor: EventReader<CursorMoved>,
) {
    let (camera, camera_transform) = q_camera.single();
    if let Some(cursor) = evr_cursor.iter().next() {
        for mut transform in q.iter_mut() {
            let point: Option<Vec3> =
                camera.screen_to_point_2d(cursor.position, &windows, &images, camera_transform);
            println!("Point {:?}", point);
            if let Some(point) = point {
                transform.translation = point;
            }
        }
    }
}
