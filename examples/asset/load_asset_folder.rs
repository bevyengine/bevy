use bevy::{asset::AssetServer, prelude::*};

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(mut asset_server: ResMut<AssetServer>) {
    asset_server.add_asset_folder("assets");
    asset_server.load_assets().expect("Assets should exist");
}
