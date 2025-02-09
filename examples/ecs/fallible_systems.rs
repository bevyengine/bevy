//! Showcases how fallible systems can be make use of rust's powerful result handling syntax.

use bevy::math::sampling::UniformMeshSampler;
use bevy::prelude::*;

use rand::distributions::Distribution;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, (setup, failing_system))
        .add_systems(PostStartup, failing_system)
        // You can set a global error handler for fallible systems.
        //
        // By default, fallible systems that return an error will panic.
        .set_systems_error_handler(|err, system| warn!("{:?} failed: {err}", system.name()))
        // You can handle individual systems by piping the output result.
        .add_systems(
            PostStartup,
            failing_system.pipe(|result: In<Result>| {
                result.0.inspect_err(|err| info!("captured error: {err}"))
            }),
        );

    // You can also set a custom error handler per `Schedule` (optionally at runtime through
    // `World::try_schedule_scope`):
    app.get_schedule_mut(PostStartup)
        .unwrap()
        .set_error_handler(|err, system| error!("{:?} failed: {err}", system.name()));

    app.run();
}

/// An example of a system that calls several fallible functions with the question mark operator.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) -> Result {
    let mut rng = rand::thread_rng();

    // Make a plane for establishing space.
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
        Transform::from_xyz(0.0, -2.5, 0.0),
    ));

    // Spawn a light:
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Spawn a camera:
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Create a new sphere mesh:
    let mut sphere_mesh = Sphere::new(1.0).mesh().ico(7)?;
    sphere_mesh.generate_tangents()?;

    // Spawn the mesh into the scene:
    let mut sphere = commands.spawn((
        Mesh3d(meshes.add(sphere_mesh.clone())),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_xyz(-1.0, 1.0, 0.0),
    ));

    // Generate random sample points:
    let triangles = sphere_mesh.triangles()?;
    let distribution = UniformMeshSampler::try_new(triangles)?;

    // Setup sample points:
    let point_mesh = meshes.add(Sphere::new(0.01).mesh().ico(3)?);
    let point_material = materials.add(StandardMaterial {
        base_color: Srgba::RED.into(),
        emissive: LinearRgba::rgb(1.0, 0.0, 0.0),
        ..default()
    });

    // Add sample points as children of the sphere:
    for point in distribution.sample_iter(&mut rng).take(10000) {
        sphere.with_child((
            Mesh3d(point_mesh.clone()),
            MeshMaterial3d(point_material.clone()),
            Transform::from_translation(point),
        ));
    }

    // Indicate the system completed successfully:
    Ok(())
}

#[derive(Resource)]
struct UninitializedResource;

fn failing_system(world: &mut World) -> Result {
    world
        .get_resource::<UninitializedResource>()
        .ok_or("Resource not initialized")?;

    Ok(())
}
