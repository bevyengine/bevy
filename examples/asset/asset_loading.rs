//! This example illustrates various ways to load assets.

use bevy::{asset::LoadedFolder, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    meshes: Res<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // By default AssetServer will load assets from inside the "assets" folder.
    // For example, the next line will load GltfAssetLabel::Primitive{mesh:0,primitive:0}.from_asset("ROOT/assets/models/cube/cube.gltf"),
    // where "ROOT" is the directory of the Application.
    //
    // This can be overridden by setting the "CARGO_MANIFEST_DIR" environment variable (see
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html)
    // to another directory. When the Application is run through Cargo, "CARGO_MANIFEST_DIR" is
    // automatically set to your crate (workspace) root directory.
    let cube_handle = asset_server.load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset("models/cube/cube.gltf"),
    );
    let sphere_handle = asset_server.load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset("models/sphere/sphere.gltf"),
    );

    // All assets end up in their Assets<T> collection once they are done loading:
    if let Some(sphere) = meshes.get(&sphere_handle) {
        // You might notice that this doesn't run! This is because assets load in parallel without
        // blocking. When an asset has loaded, it will appear in relevant Assets<T>
        // collection.
        info!("{:?}", sphere.primitive_topology());
    } else {
        info!("sphere hasn't loaded yet");
    }

    // You can load all assets in a folder like this. They will be loaded in parallel without
    // blocking. The LoadedFolder asset holds handles to each asset in the folder. These are all
    // dependencies of the LoadedFolder asset, meaning you can wait for the LoadedFolder asset to
    // fire AssetEvent::LoadedWithDependencies if you want to wait for all assets in the folder
    // to load.
    // If you want to keep the assets in the folder alive, make sure you store the returned handle
    // somewhere.
    let _loaded_folder: Handle<LoadedFolder> = asset_server.load_folder("models/torus");

    // If you want a handle to a specific asset in a loaded folder, the easiest way to get one is to call load.
    // It will _not_ be loaded a second time.
    // The LoadedFolder asset will ultimately also hold handles to the assets, but waiting for it to load
    // and finding the right handle is more work!
    let torus_handle = asset_server.load(
        GltfAssetLabel::Primitive {
            mesh: 0,
            primitive: 0,
        }
        .from_asset("models/torus/torus.gltf"),
    );

    // You can also add assets directly to their Assets<T> storage:
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::srgb(0.8, 0.7, 0.6),
        ..default()
    });

    // torus
    commands.spawn(PbrBundle {
        mesh: torus_handle,
        material: material_handle.clone(),
        transform: Transform::from_xyz(-3.0, 0.0, 0.0),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: cube_handle,
        material: material_handle.clone(),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..default()
    });
    // sphere
    commands.spawn(PbrBundle {
        mesh: sphere_handle,
        material: material_handle,
        transform: Transform::from_xyz(3.0, 0.0, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 3.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
