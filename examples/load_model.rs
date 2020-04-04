use bevy::{asset, prelude::*};

fn main() {
    asset::load_gltf("examples/assets/Box.gltf").unwrap();
    App::build().add_default_plugins().run();
}
