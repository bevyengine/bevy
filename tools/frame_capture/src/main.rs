use bevy::prelude::*;
use frame_capture::{
    basic_cube_scene,
    scene_tester::{SceneController, SceneTesterPlugin},
};

fn main() {
    App::new()
        // set create_images to true to create test image files
        // TODO use command line args?
        .insert_resource(SceneController::new(false))
        .add_plugin(SceneTesterPlugin)
        .add_plugin(basic_cube_scene::ScenePlugin)
        .run();
}
