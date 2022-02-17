use bevy_app::{App, Events, Plugin};
use bevy_ecs::{
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
                    .thread_name("IO Task Pool".to_string())
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

/// This registers the given handle with the handle
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
        .unwrap();
    let absolute_file_path = manifest_dir_path.join(Path::new(file_path).parent().unwrap());
    let asset_folder_relative_path = absolute_file_path
        .strip_prefix(asset_io.root_path())
        .unwrap();
    handle_map.handles.insert(
        asset_server.load(asset_folder_relative_path.join(path)),
        handle.clone_weak().typed::<A>(),
    );
}
