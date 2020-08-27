use bevy::prelude::*;
use simplelog::*;

fn main() {
    SimpleLogger::init(LevelFilter::Warn, Config::default()).unwrap();

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
            material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
            translation: Translation::new(-2.5, 0.0, 0.0),
            ..Default::default()
        })
        // mesh
        .spawn(PbrComponents {
            // load a mesh from binary glTF
            mesh: asset_server
                .load("assets/models/monkey/Monkey.glb")
                .unwrap(),
            // create a material for the mesh
            material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
            translation: Translation::new(0.5, 0.0, 0.0),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, 5.0, 4.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(-2.0, 2.0, 6.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });

    // We can load textures from GLTF as well. Currently, this method only loads the first mesh and first texture in the file.
    let cube = asset_server
        .load::<Mesh, _>("assets/models/cube/textured_cube.glb")
        .unwrap();
    let cube_texture = asset_server
        .load::<Texture, _>("assets/models/cube/textured_cube.glb")
        .unwrap();
    commands.spawn(PbrComponents {
        mesh: cube,
        material: materials.add(StandardMaterial {
            albedo_texture: Some(cube_texture),
            ..Default::default()
        }),
        translation: Translation::new(3.5, 0.0, 0.0),
        rotation: Rotation::from_rotation_xyz(1.2, 3.14, 0.0),
        ..Default::default()
    });
}
