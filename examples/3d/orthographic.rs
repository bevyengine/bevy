use std::f32::consts::PI;

use bevy::{input::mouse::MouseWheel, prelude::*, render::camera::OrthographicProjection};

const ROTATE_SPEED: f32 = 0.05;
const ZOOM_SPEED: f32 = 0.1;
const MIN_ZOOM: f32 = 1.0;
const MAX_ZOOM: f32 = 30.0;

struct MainCamera;

fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(zoom_system)
        .add_system(rotate_system)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>
) {
    // set up the camera
    let mut camera = OrthographicCameraBundle::new_3d();
    camera.orthographic_projection.scale = 3.0;
    camera.transform = Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y);

    // camera
    commands
        .spawn_bundle(camera)
        .insert(MainCamera);

    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cubes
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
        transform: Transform::from_xyz(1.5, 0.5, 1.5),
        ..Default::default()
    });
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.0, 1.0, 0.0).into()),
        transform: Transform::from_xyz(1.5, 0.5, -1.5),
        ..Default::default()
    });
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.0, 0.0, 1.0).into()),
        transform: Transform::from_xyz(-1.5, 0.5, 1.5),
        ..Default::default()
    });
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(1.0, 1.0, 1.0).into()),
        transform: Transform::from_xyz(-1.5, 0.5, -1.5),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(3.0, 8.0, 5.0),
        ..Default::default()
    });
}

fn zoom_system(
    mut whl: EventReader<MouseWheel>,
    mut cam: Query<(&mut Transform, &mut OrthographicProjection), With<MainCamera>>,
    windows: Res<Windows>
) {
    let delta_zoom: f32 = whl.iter().map(|e| e.y).sum();
    if delta_zoom == 0. {
        return;
    }

    if let Ok((mut cam_transform, mut projection)) = cam.single_mut() {
        let window = windows.get_primary().unwrap();
        let window_size = Vec2::new(window.width(), window.height());
        let mouse_normalized_screen_pos = window.cursor_position().unwrap() / window_size;

        let wanted_zoom = projection.scale - delta_zoom * ZOOM_SPEED;
        projection.zoom_to(mouse_normalized_screen_pos, wanted_zoom, &mut cam_transform);

        projection.scale = projection.scale.clamp(MIN_ZOOM, MAX_ZOOM);
    }
}

fn rotate_system(
    keycode: Res<Input<KeyCode>>,
    mut cam: Query<&mut Transform, With<MainCamera>>,
) {
    if let Ok(mut cam_transform) = cam.single_mut() {
        let mut direction: f32 = 0.0;

        if keycode.pressed(KeyCode::Right) {
            direction += 1.0;
        }

        if keycode.pressed(KeyCode::Left) {
            direction -= 1.0;
        }

        if direction != 0.0 {
            let zx = Vec2::new(cam_transform.translation.z, cam_transform.translation.x);
            let curr_angle = Vec2::new(1.0, 0.0).angle_between(zx);
            let curr_distance = zx.length();

            let next_angle = if curr_angle < 0.0 {
                curr_angle + 2. * PI
            } else {
                curr_angle
            } + direction * ROTATE_SPEED;

            let new_x = curr_distance * next_angle.sin();
            let new_z = curr_distance * next_angle.cos();

            cam_transform.translation.x = new_x;
            cam_transform.translation.z = new_z;
            cam_transform.look_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y);
        }
    }

}
