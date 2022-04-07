use bevy_app::{App, Plugin};
use bevy_ecs::{
    event::Events,
    schedule::SystemLabel,
    system::{NonSendMut, Res, ResMut, SystemState},
};
use bevy_tasks::{IoTaskPool, TaskPoolBuilder};
use bevy_utils::HashMap;
use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

use crate::{
    Asset, AssetEvent, AssetPlugin, AssetServer, AssetServerSettings, Assets, FileAssetIo, Handle,
    HandleUntyped,
};

/// A "debug asset app", whose sole responsibility is hot reloading assets that are
/// "internal" / compiled-in to Bevy Plugins.
pub struct DebugAssetApp(App);

impl Deref for DebugAssetApp {
    type Target = App;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DebugAssetApp {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(SystemLabel, Debug, Clone, PartialEq, Eq, Hash)]
pub struct DebugAssetAppRun;

/// Facilitates the creation of a "debug asset app", whose sole responsibility is hot reloading
/// assets that are "internal" / compiled-in to Bevy Plugins.
/// Pair with [`load_internal_asset`](crate::load_internal_asset) to load "hot reloadable" assets
/// The `debug_asset_server` feature flag must also be enabled for hot reloading to work.
/// Currently only hot reloads assets stored in the `crates` folder.
#[derive(Default)]
pub struct DebugAssetServerPlugin;
pub struct HandleMap<T: Asset> {
    pub handles: HashMap<Handle<T>, Handle<T>>,
}

impl<T: Asset> Default for HandleMap<T> {
    fn default() -> Self {
        Self {
            handles: Default::default(),
        }
    }
}

impl Plugin for DebugAssetServerPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let mut debug_asset_app = App::new();
        debug_asset_app
            .insert_resource(IoTaskPool(
                TaskPoolBuilder::default()
                    .num_threads(2)
                    .thread_name("Debug Asset Server IO Task Pool".to_string())
                    .build(),
            ))
            .insert_resource(AssetServerSettings {
                asset_folder: "crates".to_string(),
                watch_for_changes: true,
            })
            .add_plugin(AssetPlugin);
        app.insert_non_send_resource(DebugAssetApp(debug_asset_app));
        app.add_system(run_debug_asset_app);
    }
}

fn run_debug_asset_app(mut debug_asset_app: NonSendMut<DebugAssetApp>) {
    debug_asset_app.0.update();
}

pub(crate) fn sync_debug_assets<T: Asset + Clone>(
    mut debug_asset_app: NonSendMut<DebugAssetApp>,
    mut assets: ResMut<Assets<T>>,
) {
    let world = &mut debug_asset_app.0.world;
    let mut state = SystemState::<(
        Res<Events<AssetEvent<T>>>,
        Res<HandleMap<T>>,
        Res<Assets<T>>,
    )>::new(world);
    let (changed_shaders, handle_map, debug_assets) = state.get_mut(world);
    for changed in changed_shaders.iter_current_update_events() {
        let debug_handle = match changed {
            AssetEvent::Created { handle } => handle,
            AssetEvent::Modified { handle } => handle,
            AssetEvent::Removed { .. } => continue,
        };
        if let Some(handle) = handle_map.handles.get(debug_handle) {
            if let Some(debug_asset) = debug_assets.get(debug_handle) {
                assets.set_untracked(handle, debug_asset.clone());
            }
        }
    }
}

/// Uses the return type of the given loader to register the given handle with the appropriate type
/// and load the asset with the given `path` and parent `file_path`.
/// If this feels a bit odd ... thats because it is. This was built to improve the UX of the
/// `load_internal_asset` macro.
pub fn register_handle_with_loader<A: Asset>(
    _loader: fn(&'static str) -> A,
    app: &mut DebugAssetApp,
    handle: HandleUntyped,
    file_path: &str,
    path: &'static str,
) {
    let mut state = SystemState::<(ResMut<HandleMap<A>>, Res<AssetServer>)>::new(&mut app.world);
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let manifest_dir_path = Path::new(&manifest_dir);
    let (mut handle_map, asset_server) = state.get_mut(&mut app.world);
    let asset_io = asset_server
        .asset_io()
        .downcast_ref::<FileAssetIo>()
        .expect("The debug AssetServer only works with FileAssetIo-backed AssetServers");
    let absolute_file_path = manifest_dir_path.join(
        Path::new(file_path)
            .parent()
            .expect("file path must have a parent"),
    );
    let asset_folder_relative_path = absolute_file_path
        .strip_prefix(asset_io.root_path())
        .expect("The AssetIo root path should be a prefix of the absolute file path");
    handle_map.handles.insert(
        asset_server.load(asset_folder_relative_path.join(path)),
        handle.clone_weak().typed::<A>(),
    );
}
