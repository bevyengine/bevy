use bevy::{
    asset::{AssetIo, AssetIoError},
    prelude::*,
    utils::BoxedFuture,
};
use std::path::{Path, PathBuf};

/// A custom asset io implementation that simply defers to the platform default
/// implementation.
///
/// This can be used as a starting point for developing a useful implementation
/// that can defer to the default when needed.
struct CustomAssetIo(Box<dyn AssetIo>);

impl AssetIo for CustomAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        println!("load_path({:?})", path);
        self.0.load_path(path)
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        println!("read_directory({:?})", path);
        self.0.read_directory(path)
    }

    fn is_directory(&self, path: &Path) -> bool {
        println!("is_directory({:?})", path);
        self.0.is_directory(path)
    }

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        println!("watch_path_for_changes({:?})", path);
        self.0.watch_path_for_changes(path)
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        println!("watch_for_changes()");
        self.0.watch_for_changes()
    }
}

/// A plugin used to execute the override of the asset io
struct CustomAssetIoPlugin;

impl Plugin for CustomAssetIoPlugin {
    fn build(&self, app: &mut AppBuilder) {
        // must get a hold of the task pool in order to create the asset server

        let task_pool = app
            .resources()
            .get::<bevy::tasks::IoTaskPool>()
            .expect("`IoTaskPool` resource not found.")
            .0
            .clone();

        let asset_io = {
            // the platform default asset io requires a reference to the app
            // builder to find its configuration

            let default_io = bevy::asset::create_platform_default_asset_io(app);

            // create the custom asset io instance

            CustomAssetIo(default_io)
        };

        // the asset server is constructed and added the resource manager

        app.add_resource(AssetServer::new(asset_io, task_pool));
    }
}

fn main() {
    App::build()
        .add_plugins_with(DefaultPlugins, |group| {
            // the custom asset io plugin must be inserted in-between the
            // `CorePlugin' and `AssetPlugin`. It needs to be after the
            // CorePlugin, so that the IO task pool has already been constructed.
            // And it must be before the `AssetPlugin` so that the asset plugin
            // doesn't create another instance of an assert server. In general,
            // the AssetPlugin should still run so that other aspects of the
            // asset system are initialized correctly.
            group.add_before::<bevy::asset::AssetPlugin, _>(CustomAssetIoPlugin)
        })
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    commands: &mut Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    commands
        .spawn(Camera2dBundle::default())
        .spawn(SpriteBundle {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}
