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
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // You can load all assets in a folder like this. They will be loaded in parallel without blocking
    asset_server
        .load_asset_folder("assets/models/monkey")
        .unwrap();

    // Then any asset in the folder can be accessed like this:
    let monkey_handle = asset_server
        .get_handle("assets/models/monkey/Monkey.gltf")
        .unwrap();

    // You can load individual assets like this:
    let cube_handle = asset_server.load("assets/models/cube/cube.gltf").unwrap();

    // Assets are loaded in the background by default, which means they might not be available immediately after calling load().
    // If you need immediate access you can load assets synchronously like this:
    let sphere_handle = asset_server
        .load_sync(&mut meshes, "assets/models/sphere/sphere.gltf")
        .unwrap();
    // All assets end up in their Assets<T> collection once they are done loading:
    let sphere = meshes.get(&sphere_handle).unwrap();
    println!("{:?}", sphere.primitive_topology);

    // You can also add assets directly to their Assets<T> storage:
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    // Add entities to the world:
    commands
        // monkey
        .spawn(PbrComponents {
            mesh: monkey_handle,
            material: material_handle,
            transform: Transform::from_translation(Vec3::new(-3.0, 0.0, 0.0)),
            ..Default::default()
        })
        // cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: material_handle,
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
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(0.0, 3.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
