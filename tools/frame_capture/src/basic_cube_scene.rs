use bevy::prelude::*;

use crate::scene_tester::{SceneController, SceneName, SceneState, SceneTesterPlugin};

pub fn run() {
    App::new()
        .insert_resource(SceneName(String::from("basic_cube_scene")))
        .add_plugin(SceneTesterPlugin)
        .add_startup_system(scene)
        .run();
}

fn scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut scene_controller: ResMut<SceneController>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 0.25 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::rgb(0.7, 0.7, 0.7),
        reflectance: 0.02,
        unlit: false,
        ..default()
    });

    commands.spawn_bundle(PbrBundle {
        mesh: cube_handle,
        material: cube_material_handle,
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        ..default()
    });

    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..default()
    });

    scene_controller.0 = SceneState::Render(15);
}
