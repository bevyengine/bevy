use bevy::prelude::*;
use frame_capture::{
    scene_2d_shapes, scene_basic_cube,
    scene_tester::{SceneController, SceneTesterPlugin},
};

fn main() {
    // set create_images to true to create test image files
    // TODO use command line args?
    let create_images = false;

    App::new()
        .insert_resource(SceneController::new(create_images))
        .add_plugin(SceneTesterPlugin)
        .add_plugin(scene_basic_cube::ScenePlugin)
        .run();

    App::new()
        .insert_resource(SceneController::new(create_images))
        .add_plugin(SceneTesterPlugin)
        .add_plugin(scene_2d_shapes::ScenePlugin)
        .run();
}
