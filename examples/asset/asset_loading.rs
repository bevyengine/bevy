//! This example illustrates various ways to load assets.

use bevy::prelude::*;

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
    // For example, the next line will load "ROOT/assets/models/cube/cube.gltf#Mesh0/Primitive0",
    // where "ROOT" is the directory of the Application.
    //
    // This can be overridden by setting the "CARGO_MANIFEST_DIR" environment variable (see
    // https://doc.rust-lang.org/cargo/reference/environment-variables.html)
    // to another directory. When the Application is run through Cargo, "CARGO_MANIFEST_DIR" is
    // automatically set to your crate (workspace) root directory.
    let cube_handle = asset_server.load("models/cube/cube.gltf#Mesh0/Primitive0");
    let sphere_handle = asset_server.load("models/sphere/sphere.gltf#Mesh0/Primitive0");

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
    // blocking
    let _scenes: Vec<HandleUntyped> = asset_server.load_folder("models/monkey").unwrap();

    // Then any asset in the folder can be accessed like this:
    let monkey_handle = asset_server.get_handle("models/monkey/Monkey.gltf#Mesh0/Primitive0");

    // You can also add assets directly to their Assets<T> storage:
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.8, 0.7, 0.6),
        ..default()
    });

    // monkey
    commands.spawn(PbrBundle {
        mesh: monkey_handle,
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
