use bevy::{prelude::*, render::camera::Camera, render::primitives::Plane};

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(follow)
        .run();
}

#[derive(Component)]
struct Follow;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..Default::default()
        })
        .insert(Follow);
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn follow(
    mut q: Query<&mut Transform, With<Follow>>,
    q_camera: Query<(&Camera, &GlobalTransform)>,
    windows: Res<Windows>,
    images: Res<Assets<Image>>,
    mut evr_cursor: EventReader<CursorMoved>,
) {
    // Assumes there is at least one camera
    let (camera, camera_transform) = q_camera.iter().next().unwrap();
    if let Some(cursor) = evr_cursor.iter().next() {
        for mut transform in q.iter_mut() {
            let point: Option<Vec3> = camera.screen_to_point_on_plane(
                cursor.position,
                Plane::new(Vec4::new(0., 1., 0., 1.)),
                &windows,
                &images,
                camera_transform,
            );
            if let Some(point) = point {
                transform.translation = point + Vec3::new(0., 0.5, 0.);
            }
        }
    }
}
