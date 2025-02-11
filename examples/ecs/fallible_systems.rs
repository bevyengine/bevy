//! Showcases how fallible systems and observers can make use of Rust's powerful result handling
//! syntax.

use bevy::ecs::world::DeferredWorld;
use bevy::math::sampling::UniformMeshSampler;
use bevy::prelude::*;

use rand::distributions::Distribution;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // Fallible systems and observers can pipe the result into another system.
        .add_observer(fallible_observer.pipe(log));

    #[cfg(feature = "bevy_mesh_picking_backend")]
    app.add_plugins(MeshPickingPlugin);

    app.run();
}

/// An example of a system that calls several fallible functions with the question mark operator.
///
/// See: <https://doc.rust-lang.org/reference/expressions/operator-expr.html#the-question-mark-operator>
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

// Observer systems can also return a `Result`.
fn fallible_observer(
    trigger: Trigger<Pointer<Move>>,
    mut world: DeferredWorld,
    mut step: Local<f32>,
) -> Result {
    let mut transform = world
        .get_mut::<Transform>(trigger.target)
        .ok_or("No transform found.")?;

    *step = if transform.translation.x > 3. {
        -0.1
    } else if transform.translation.x < -3. || *step == 0. {
        0.1
    } else {
        *step
    };

    transform.translation.x += *step;

    Ok(())
}

// Take the output of a fallible system as an input in this system, and log it.
fn log(In(result): In<Result>) {
    let Err(error) = result else {
        return;
    };

    error!(error, "Observer failed.");
}
