use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    mut asset_server: ResMut<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // You can load all assets in a folder like this. They will be loaded in parallel without blocking
    asset_server.load_asset_folder("assets/models/monkey").unwrap();

    // Then any asset in the folder can be accessed like this:
    let monkey_handle = asset_server
        .get_handle("assets/models/monkey/Monkey.gltf")
        .unwrap();

    // You can load individual assets like this:
    let cube_handle = asset_server
        .load("assets/models/cube/cube.gltf")
        .unwrap();
   
    // Assets are loaded in the background by default, which means they might not be available immediately after calling load().
    // If you need immediate access you can load assets synchronously like this:
    let sphere_handle = asset_server.load_sync(&mut meshes, "assets/models/sphere/sphere.gltf").unwrap();
    // All assets end up in their Assets<T> collection once they are done loading:
    let sphere = meshes.get(&sphere_handle).unwrap();
    println!("{:?}", sphere.primitive_topology);

    // You can also add assets directly to their Assets<T> storage:
    let material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    // Add entities to the world:
    command_buffer
        .build()
        // monkey
        .add_entity(MeshEntity {
            mesh: monkey_handle,
            material: material_handle,
            translation: Translation::new(-3.0, 0.0, 0.0),
            ..Default::default()
        })
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: material_handle,
            translation: Translation::new(0.0, 0.0, 0.0),
            ..Default::default()
        })
        // sphere
        .add_entity(MeshEntity {
            mesh: sphere_handle,
            material: material_handle,
            translation: Translation::new(3.0, 0.0, 0.0),
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // camera
        .add_entity(PerspectiveCameraEntity {
            local_to_world: LocalToWorld::new_sync_disabled(Mat4::look_at_rh(
                Vec3::new(0.0, -10.0, 3.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
