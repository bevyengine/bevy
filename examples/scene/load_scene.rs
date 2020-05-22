use bevy::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    App::build()
        .add_default_plugins()
        // Registering components informs Bevy that they exist. This allows them to be used when loading/saving scenes
        // This step is only required if you want to load/save your components.
        .register_component::<Test>()
        .register_component::<Foo>()
        // .add_startup_system(setup)
        .add_startup_system(load_scene)
        .run();
}

#[derive(Serialize, Deserialize, Clone)]
struct Test {
    pub x: f32,
    pub y: f32,
}

#[derive(Serialize, Deserialize, Clone)]
struct Foo {
    pub value: String,
}

fn load_scene(_world: &mut World, resources: &mut Resources) {
    let asset_server = resources.get::<AssetServer>().unwrap();
    let mut scenes = resources.get_mut::<Assets<Scene>>().unwrap();

    let scene_handle: Handle<Scene> = asset_server
        .load_sync(&mut scenes, "assets/scene/load_scene_example.scn")
        .unwrap();
    let _scene= scenes.get(&scene_handle).unwrap();
    // world.merge(scene)
}