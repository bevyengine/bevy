use bevy::animation::Animator;
use bevy::asset::AssetServerSettings;
use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    let sphere = meshes.add(Mesh::from(shape::Icosphere {
        radius: 1.0,
        subdivisions: 5,
    }));

    let entity = commands
        .spawn(PbrBundle {
            mesh: cube.clone(),
            transform: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
            material: materials.add(Color::rgb(0.1, 0.05, 0.0).into()),
            ..Default::default()
        })
        .with_;

    commands
        // plane
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 20.0 })),
            transform: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
            material: materials.add(Color::rgb(0.1, 0.05, 0.0).into()),
            ..Default::default()
        })
        // light
        .spawn(LightBundle {
            transform: Transform::from_translation(Vec3::new(4.0, 8.0, 4.0)),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        })
}

fn anim_set(asset_server: Res<AssetServer>, mut animators_query: Query<(&mut Animator,)>) {
    // Load animations and set them to the animator
    for (mut animator,) in animators_query.iter_mut() {
        if animator.clips().len() == 0 {
            animator.add_layer(
                asset_server.load("models/character_medium/idle.gltf#Anim0"),
                1.0,
            );
            animator.add_layer(
                asset_server.load("models/character_medium/run.gltf#Anim0"),
                1.0,
            );
        }
    }
}
