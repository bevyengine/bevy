//! This module tests basic asset loading.

extern crate core;

use bevy::prelude::*;
use bevy_internal::asset::{Asset, LoadState};
use bevy_internal::gltf::GltfPlugin;
use bevy_internal::log::LogPlugin;
use bevy_internal::render::mesh::MeshPlugin;
use bevy_internal::render::render_resource::ShaderLoader;
use bevy_internal::scene::SceneLoader;
use std::time::Instant;

const ASSET_LOADER_TIMEOUT: u128 = 500;

#[cfg(feature = "png")]
#[test]
fn load_png() {
    let mut app = setup_test_app();

    let image: &Image = assert_asset_loads(&mut app, "load_tests/colors.png");
    assert_image_loaded_properly(image);
}

#[cfg(feature = "jpeg")]
#[test]
fn load_jpeg() {
    let mut app = setup_test_app();

    let image: &Image = assert_asset_loads(&mut app, "load_tests/colors.jpg");
    assert_image_loaded_properly(image);
}

#[cfg(feature = "bmp")]
#[test]
fn load_bmp() {
    let mut app = setup_test_app();

    let image: &Image = assert_asset_loads(&mut app, "load_tests/colors.bmp");
    assert_image_loaded_properly(image);
}

#[cfg(feature = "tga")]
#[test]
fn load_tga() {
    let mut app = setup_test_app();

    let image: &Image = assert_asset_loads(&mut app, "load_tests/colors.tga");
    assert_image_loaded_properly(image);
}

fn assert_image_loaded_properly(image: &Image) {
    assert_eq!(image.size(), Vec2::new(100., 50.));
}

#[cfg(feature = "bevy_gltf")]
#[test]
fn load_gltf() {
    let mut app = setup_test_app();

    let cube: &Mesh = assert_asset_loads(&mut app, "models/cube/cube.gltf#Mesh0/Primitive0");
    assert_eq!(cube.count_vertices(), 24);
}

fn setup_test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        // TODO (Wcubed 2023-01-22): somehow enable logging without using LogPlugin.
        //      because the log plugin registers globally. Meaning it will try (and fail) to
        //      register for each test. And the log output will end up in the wrong test output.
        .add_plugin(LogPlugin::default())
        .add_plugin(AssetPlugin::default())
        .add_plugin(ImagePlugin::default());

    #[cfg(feature = "bevy_gltf")]
    {
        app.add_asset::<Scene>()
            .init_asset_loader::<SceneLoader>()
            .add_asset::<Shader>()
            .init_asset_loader::<ShaderLoader>()
            .add_plugin(MeshPlugin)
            .add_plugin(MaterialPlugin::<StandardMaterial>::default())
            .add_plugin(GltfPlugin::default());
    }

    app
}

/// Convenience function that will return once the desired asset is loaded.
/// Panics if the asset loading fails for any reason.
fn assert_asset_loads<'a, T: Asset>(app: &'a mut App, path: &str) -> &'a T {
    let start = Instant::now();

    let asset_server: &AssetServer = app.world.resource();
    let handle: Handle<T> = asset_server.load(path);

    loop {
        if start.elapsed().as_millis() > ASSET_LOADER_TIMEOUT {
            panic!("Loading asset with path `{path}` timed out.");
        }

        app.update();

        let asset_server: &AssetServer = app.world.resource();
        match asset_server.get_load_state(&handle) {
            LoadState::NotLoaded | LoadState::Loading => {
                // Not loaded yet, wait another cycle.
            }
            LoadState::Loaded => {
                // Ok, continue.
                break;
            }
            LoadState::Failed => {
                panic!("Asset with path `{path}` should have loaded successfully, but it failed.");
            }
            LoadState::Unloaded => {
                panic!("Asset with path `{path}` should have loaded successfully, but it was unloaded for some reason.");
            }
        }
    }

    let assets: &Assets<T> = app.world.resource();

    assets.get(&handle).unwrap_or_else(|| {
        panic!("Asset `{path}` was loaded successfully, but calling `get()` returned `None`.")
    })
}
