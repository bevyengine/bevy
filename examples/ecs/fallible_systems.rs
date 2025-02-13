//! Showcases how fallible systems and observers can make use of Rust's powerful result handling
//! syntax.

use bevy::ecs::world::DeferredWorld;
use bevy::math::sampling::UniformMeshSampler;
use bevy::prelude::*;

use rand::distributions::Distribution;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn main() {
    let mut app = App::new();

    app.add_plugins(DefaultPlugins);

    #[cfg(feature = "bevy_mesh_picking_backend")]
    app.add_plugins(MeshPickingPlugin);

    // Fallible systems can be used the same way as regular systems. The only difference is they
    // return a `Result<(), Box<dyn Error>>` instead of a `()` (unit) type. Bevy will handle both
    // types of systems the same way, except for the error handling.
    app.add_systems(Startup, (setup, failing_system));

    // By default, fallible systems that return an error will panic.
    //
    // We can change this by setting a custom error handler. This can be done globally for all
    // systems in a given `App`. Here we set the global error handler using one of the built-in
    // error handlers. Bevy provides built-in handlers for `panic`, `error`, `warn`, `info`,
    // `debug`, `trace` and `ignore`.
    app.set_system_error_handler(bevy::ecs::result::warn);

    // Additionally, you can set a custom error handler per `Schedule`. This will take precedence
    // over the global error handler.
    //
    // In this instance we provide our own non-capturing closure that coerces to the expected error
    // handler function pointer:
    //
    //     fn(bevy_ecs::result::Error, bevy_ecs::result::SystemErrorContext)
    //
    app.add_systems(PostStartup, failing_system)
        .get_schedule_mut(PostStartup)
        .unwrap()
        .set_error_handler(|err, ctx| error!("{} failed: {err}", ctx.name));

    // Individual systems can also be handled by piping the output result:
    app.add_systems(
        PostStartup,
        failing_system.pipe(|result: In<Result>| {
            let _ = result.0.inspect_err(|err| info!("captured error: {err}"));
        }),
    );

    // Fallible observers are also sypported.
    app.add_observer(fallible_observer);

    // If we run the app, we'll see the following output at startup:
    //
    //  WARN Encountered an error in system `fallible_systems::failing_system`: "Resource not initialized"
    // ERROR fallible_systems::failing_system failed: Resource not initialized
    //  INFO captured error: Resource not initialized
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
    let mut seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);

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
    for point in distribution.sample_iter(&mut seeded_rng).take(10000) {
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

#[derive(Resource)]
struct UninitializedResource;

fn failing_system(world: &mut World) -> Result {
    world
        // `get_resource` returns an `Option<T>`, so we use `ok_or` to convert it to a `Result` on
        // which we can call `?` to propagate the error.
        .get_resource::<UninitializedResource>()
        // We can provide a `str` here because `Box<dyn Error>` implements `From<&str>`.
        .ok_or("Resource not initialized")?;

    Ok(())
}
