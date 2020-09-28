use bevy::prelude::*;

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
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // add entities to the world
    commands
        // mesh
        .spawn(PbrComponents {
            // load a mesh from glTF
            mesh: asset_server
                .load("assets/models/monkey/Monkey.gltf")
                .unwrap(),
            // create a material for the mesh
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_translation(Vec3::new(-1.5, 0.0, 0.0)),
            ..Default::default()
        })
        // mesh
        .spawn(PbrComponents {
            // load a mesh from binary glTF
            mesh: asset_server
                .load("assets/models/monkey/Monkey.glb")
                .unwrap(),
            // create a material for the mesh
            material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
            transform: Transform::from_translation(Vec3::new(1.5, 0.0, 0.0)),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            transform: Transform::from_translation(Vec3::new(4.0, 5.0, 4.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::from_translation(Vec3::new(-2.0, 2.0, 6.0)).looking_at_origin(),
            ..Default::default()
        });
}
