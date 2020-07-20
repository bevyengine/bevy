use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, PrintDiagnosticsPlugin},
    prelude::*,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

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
        material.albedo += Color::rgb(-time.delta_seconds, -time.delta_seconds, time.delta_seconds);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let plane_handle = meshes.add(Mesh::from(shape::Plane { size: 10.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });
    let plane_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.1, 0.2, 0.1),
        ..Default::default()
    });

    commands
        // plane
        .spawn(PbrComponents {
            mesh: plane_handle,
            material: plane_material_handle,
            ..Default::default()
        })
        // cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: cube_material_handle,
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(3.0, 5.0, -8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });

    let mut rng = StdRng::from_entropy();
    for _ in 0..10000 {
        let spawned_material_handle = materials.add(StandardMaterial {
            albedo: Color::rgb(
                rng.gen_range(0.0, 1.0),
                rng.gen_range(0.0, 1.0),
                rng.gen_range(0.0, 1.0),
            ),
            ..Default::default()
        });
        commands.spawn(PbrComponents {
            mesh: cube_handle,
            material: spawned_material_handle,
            translation: Translation::new(
                rng.gen_range(-50.0, 50.0),
                0.0,
                rng.gen_range(-50.0, 50.0),
            ),
            ..Default::default()
        });
    }
}
