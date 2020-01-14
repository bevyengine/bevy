use bevy::{prelude::*, asset};

fn main() {
    asset::load_gltf("examples/assets/Box.gltf").unwrap();
    AppBuilder::new().add_defaults().run();
}
