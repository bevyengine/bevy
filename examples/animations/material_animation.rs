use bevy::animation::{AddAnimated, Animator, Clip};
use bevy::asset::AssetServerSettings;
use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .register_animated_asset::<StandardMaterial>()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut clips: ResMut<Assets<Clip>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // TODO: create clip here

    let mut animator = Animator::default();
    // TODO: add clip

    let entity = commands
        .spawn(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Icosphere {
                radius: 1.0,
                subdivisions: 5,
            })),
            transform: Transform::from_translation(Vec3::new(0.0, -1.0, 0.0)),
            material: materials.add(Color::rgb(0.1, 0.05, 0.0).into()),
            ..Default::default()
        })
        .with(animator);

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
        .spawn(PerspectiveCameraBundle {
            transform: Transform::from_matrix(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
