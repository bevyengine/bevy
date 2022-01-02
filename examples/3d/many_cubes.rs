use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const WIDTH: usize = 100;
    const HEIGHT: usize = 100;
    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let material = materials.add(StandardMaterial {
        base_color: Color::PINK,
        ..Default::default()
    });
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            // cube
            commands.spawn_bundle(PbrBundle {
                mesh: mesh.clone(),
                material: material.clone(),
                transform: Transform::from_xyz((x as f32) * 2.0, (y as f32) * 2.0, 0.0),
                ..Default::default()
            });
        }
    }

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(80.0, 80.0, 300.0),
        ..Default::default()
    });
}
