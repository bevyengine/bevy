use bevy::{gltf, prelude::*};

fn main() {
    let mesh = gltf::load_gltf("examples/assets/Box.gltf").unwrap().unwrap();
    // App::build().add_default_plugins().run();
}
