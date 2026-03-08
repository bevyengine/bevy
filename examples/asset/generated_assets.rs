//! Shows how to generate and store assets at runtime.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, generate_mesh_system.run_if(run_once))
        .run();
}

fn setup(
    mut commands: Commands,
    mut asset_commands: AssetCommands,
    asset_server: Res<AssetServer>,
) {
    commands.spawn((Camera3d::default(), Transform::from_xyz(0.0, 0.0, 5.0)));

    commands.spawn((
        DirectionalLight::default(),
        Transform::default().looking_to(Dir3::new(Vec3::new(-1.0, -1.0, -1.0)).unwrap(), Dir3::Y),
    ));

    // The simplest way to generate an asset is to spawn it directly with `AssetCommands`.
    let material_handle = asset_commands.spawn_asset(StandardMaterial::default());

    commands.spawn((
        Transform::from_xyz(-2.0, 0.0, 0.0),
        MeshMaterial3d(material_handle.clone()),
        // Alternatively, `add_async` creates a task that runs your async function. Once it
        // completes, the asset is inserted into the returned handle. This is "deferred" meaning
        // that the asset may take a frame to be added after the task completes.
        Mesh3d(asset_server.add_async(generate_mesh_async())),
    ));

    // The last way to generate assets is to reserve a handle, and then use
    // `AssetCommands::insert_asset` to populate the asset later. In this example, the
    // `generate_mesh_system` system runs to populate the mesh.
    let mesh_handle = asset_commands.reserve_handle();
    commands.insert_resource(HandleToGenerate(mesh_handle.clone()));
    commands.spawn((
        Transform::from_xyz(2.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_x(50.0f32.to_radians())),
        Mesh3d(mesh_handle),
        MeshMaterial3d(material_handle),
    ));
}

async fn generate_mesh_async() -> Result<Mesh, std::io::Error> {
    // This mesh could take a while to generate. It could even take several frames (though in this
    // example it should be ~instant).

    Ok(Mesh::from(Cone::new(1.0, 2.0)))
}

#[derive(Resource)]
struct HandleToGenerate(Handle<Mesh>);

/// This system runs once to populate the handle in [`HandleToGenerate`].
///
/// This generates a runtime mesh. Since it's a system, it can use other data in the world to
/// generate the asset!
fn generate_mesh_system(
    mut asset_commands: AssetCommands,
    handle_to_generate: Res<HandleToGenerate>,
) {
    let mesh = Mesh::from(Torus::new(0.8, 1.2));
    asset_commands.insert_asset(&handle_to_generate.0, mesh);
}
