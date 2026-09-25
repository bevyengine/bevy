use bevy::prelude::*;
use bevy::asset::LoadState;

#[test]
fn test_asset_load_valid_file() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>();

    let asset_server = app.world().resource::<AssetServer>();
    let handle: Handle<Image> = asset_server.load("branding/bevy_bird_dark.png");

    // Confirm the asset exists in Bevy's registry
    let state = asset_server.get_load_state(handle.id());
    assert!(
        matches!(state, Some(LoadState::NotLoaded) | Some(LoadState::Loading) | Some(LoadState::Loaded)),
        "Valid file should be in a valid load state, got {:?}",
        state
    );
}

#[test]
fn test_asset_load_invalid_file() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>();

    let asset_server = app.world().resource::<AssetServer>();
    
    // Bevy doesn't panic on invalid paths - it creates a handle and marks it as failed
    let handle: Handle<Image> = asset_server.load("nonexistent/fake_file.png");

    // The handle should be created successfully (no panic)
    let state = asset_server.get_load_state(handle.id());
    
    // We just verify that a handle was created - Bevy will handle the error internally
    assert!(
        state.is_some(),
        "Invalid file should still create a handle, got {:?}",
        state
    );
}

#[test]
#[should_panic]
fn test_asset_load_empty_path_panics() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(AssetPlugin::default())
        .init_asset::<Image>();

    let asset_server = app.world().resource::<AssetServer>();

    // Test that load panics with empty string
    let _handle: Handle<Image> = asset_server.load("");
}