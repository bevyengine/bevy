use bevy::{
    prelude::*,
    window::{PresentMode, RequestRedraw},
    winit::{ControlFlow, WinitConfig},
};

/// This example illustrates how to run in low power mode, useful for making desktop applications.
/// The app will only update when there is an event (resize, mouse input, etc.), or you send a
/// redraw request.
fn main() {
    App::new()
        .insert_resource(WinitConfig {
            control_flow: ControlFlow::Wait,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotate)
        // Try uncommenting this system to manually request a redraw every frame:
        //.add_system(request_redraw)
        .run();
}

fn request_redraw(mut event: EventWriter<RequestRedraw>) {
    event.send(RequestRedraw);
}

fn rotate(mut cube_transform: Query<&mut Transform, With<Rotator>>) {
    for mut transform in cube_transform.iter_mut() {
        transform.rotate(Quat::from_rotation_x(0.05));
        transform.rotate(Quat::from_rotation_y(0.05));
        transform.rotate(Quat::from_rotation_z(0.05));
    }
}

#[derive(Component)]
struct Rotator;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut event: EventWriter<RequestRedraw>,
) {
    // Spawn a big block of cubes
    for i in -5..5 {
        for j in -5..5 {
            for k in -5..5 {
                commands
                    .spawn_bundle(PbrBundle {
                        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
                        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
                        transform: Transform::from_xyz(i as f32, j as f32, k as f32),
                        ..Default::default()
                    })
                    .insert(Rotator);
            }
        }
    }
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-10.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
    event.send(RequestRedraw);
}
