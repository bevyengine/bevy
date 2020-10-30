use bevy::prelude::*;

/// This example illustrates various ways to load assets
fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    meshes: Res<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // You can load individual assets like this:
    let cube_handle = asset_server.load("models/cube/cube.gltf#Mesh0/Primitive0");
    let sphere_handle = asset_server.load("models/sphere/sphere.gltf#Mesh0/Primitive0");

    // All assets end up in their Assets<T> collection once they are done loading:
    if let Some(sphere) = meshes.get(&sphere_handle) {
        // You might notice that this doesn't run! This is because assets load in parallel without blocking.
        // When an asset has loaded, it will appear in relevant Assets<T> collection.
        println!("{:?}", sphere.primitive_topology);
    } else {
        println!("sphere hasn't loaded yet");
    }

    // You can load all assets in a folder like this. They will be loaded in parallel without blocking
    let _scenes: Vec<HandleUntyped> = asset_server.load_folder("models/monkey").unwrap();

    // Then any asset in the folder can be accessed like this:
    let monkey_handle = asset_server.get_handle("models/monkey/Monkey.gltf#Mesh0/Primitive0");

    // You can also add assets directly to their Assets<T> storage:
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.8, 0.7, 0.6),
        ..Default::default()
    });

    // Add entities to the world:
    commands
        // monkey
        .spawn(PbrComponents {
            mesh: monkey_handle,
            material: material_handle.clone(),
            transform: Transform::from_translation(Vec3::new(-3.0, 0.0, 0.0)),
            ..Default::default()
        })
        // cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: material_handle.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        // sphere
        .spawn(PbrComponents {
            mesh: sphere_handle,
            material: material_handle,
            transform: Transform::from_translation(Vec3::new(3.0, 0.0, 0.0)),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, 4.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::from_translation(Vec3::new(0.0, 3.0, 10.0))
                .looking_at(Vec3::default(), Vec3::unit_y()),
            ..Default::default()
        });
}
