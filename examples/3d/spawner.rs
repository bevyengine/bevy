use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

/// This example spawns a large number of cubes, each with its own changing position and material
/// This is intended to be a stress test of bevy's ability to render many objects with different properties
/// For the best results, run it in release mode: ```cargo run --example spawner --release
/// NOTE: Bevy still has a number of optimizations to do in this area. Expect the performance here to go way up in the future
fn main() {
    App::build()
        .add_default_plugins()
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(PrintDiagnosticsPlugin::default())
        .add_startup_system(setup.system())
        .add_system(move_cubes.system())
        .run();
}

fn move_cubes(
    time: Res<Time>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<(&mut Translation, &Handle<StandardMaterial>)>,
) {
    for (mut translation, material_handle) in &mut query.iter() {
        let material = materials.get_mut(&material_handle).unwrap();
        translation.0 += Vec3::new(1.0, 0.0, 0.0) * time.delta_seconds;
        material.albedo =
            Color::BLUE * Vec3::splat((3.0 * time.seconds_since_startup as f32).sin());
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(0.0, 15.0, 150.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });

    let mut rng = StdRng::from_entropy();
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    for _ in 0..10000 {
        commands.spawn(PbrComponents {
            mesh: cube_handle,
            material: materials.add(StandardMaterial {
                albedo: Color::rgb(
                    rng.gen_range(0.0, 1.0),
                    rng.gen_range(0.0, 1.0),
                    rng.gen_range(0.0, 1.0),
                ),
                ..Default::default()
            }),
            translation: Translation::new(
                rng.gen_range(-50.0, 50.0),
                rng.gen_range(-50.0, 50.0),
                0.0,
            ),
            ..Default::default()
        });
    }
}
