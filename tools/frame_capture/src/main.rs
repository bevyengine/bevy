use bevy::prelude::*;
use frame_capture::{
    scene_2d_shapes, scene_basic_cube,
    scene_tester::{SceneController, SceneTesterPlugin},
};

fn main() {
    // set create_images to true to create test image files
    // TODO use command line args?
    let create_images = true;

    App::new()
        .insert_resource(SceneController::new(create_images))
        .add_plugins((SceneTesterPlugin, scene_basic_cube::ScenePlugin))
        .run();

    // TODO: After updating to bevy 0.11 this doesn't ever run for some reason:
    App::new()
        .insert_resource(SceneController::new(create_images))
        .add_plugins((SceneTesterPlugin, scene_2d_shapes::ScenePlugin))
        .run();
}
