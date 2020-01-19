use bevy::{asset, prelude::*};

fn main() {
    asset::load_gltf("examples/assets/Box.gltf").unwrap();
    AppBuilder::new().add_defaults_legacy().run();
}
