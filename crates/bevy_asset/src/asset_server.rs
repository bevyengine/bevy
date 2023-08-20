use crate::{
    path::{AssetPath, AssetPathId, SourcePathId},
    Asset, AssetIo, AssetIoError, AssetLifecycle, AssetLifecycleChannel, AssetLifecycleEvent,
    AssetLoader, Assets, Handle, HandleId, HandleUntyped, LabelId, LoadContext, LoadState,
    RefChange, RefChangeChannel, SourceInfo, SourceMeta,
};
use anyhow::Result;
use bevy_ecs::system::{Res, ResMut, Resource};
use bevy_log::warn;
use bevy_tasks::IoTaskPool;
use bevy_utils::{Entry, HashMap, Uuid};
use crossbeam_channel::TryRecvError;
use parking_lot::{Mutex, RwLock};
use std::{path::Path, sync::Arc};
use thiserror::Error;

/// Errors that occur while loading assets with an [`AssetServer`].
#[derive(Error, Debug)]
pub enum AssetServerError {
    /// Asset folder is not a directory.
    #[error("asset folder path is not a directory: {0}")]
    AssetFolderNotADirectory(String),

    /// No asset loader was found for the specified extensions.
    #[error("no `AssetLoader` found{}", format_missing_asset_ext(.extensions))]
    MissingAssetLoader {
        /// The list of extensions detected on the asset source path that failed to load.
        ///
        /// The list may be empty if the asset path is invalid or doesn't have an extension.
        extensions: Vec<String>,
    },

    /// The handle type does not match the type of the loaded asset.
    #[error("the given type does not match the type of the loaded asset")]
    IncorrectHandleType,

    /// Encountered an error while processing an asset.
    #[error("encountered an error while loading an asset: {0}")]
    AssetLoaderError(anyhow::Error),

    /// Encountered an error while reading an asset from disk.
    #[error("encountered an error while reading an asset: {0}")]
    AssetIoError(#[from] AssetIoError),
}

fn format_missing_asset_ext(exts: &[String]) -> String {
    if !exts.is_empty() {
        format!(
            " for the following extension{}: {}",
            if exts.len() > 1 { "s" } else { "" },
            exts.join(", ")
        )
    } else {
        String::new()
    }
}

#[derive(Default)]
pub(crate) struct AssetRefCounter {
    pub(crate) channel: Arc<RefChangeChannel>,
    pub(crate) ref_counts: Arc<RwLock<HashMap<HandleId, usize>>>,
    pub(crate) mark_unused_assets: Arc<Mutex<Vec<HandleId>>>,
}

#[derive(Clone)]
enum MaybeAssetLoader {
    Ready(Arc<dyn AssetLoader>),
    Pending {
        sender: async_channel::Sender<()>,
        receiver: async_channel::Receiver<()>,
    },
}

/// Internal data for the asset server.
///
/// [`AssetServer`] is the public API for interacting with the asset server.
pub struct AssetServerInternal {
    pub(crate) asset_io: Box<dyn AssetIo>,
    pub(crate) asset_ref_counter: AssetRefCounter,
    pub(crate) asset_sources: Arc<RwLock<HashMap<SourcePathId, SourceInfo>>>,
    pub(crate) asset_lifecycles: Arc<RwLock<HashMap<Uuid, Box<dyn AssetLifecycle>>>>,
    loaders: RwLock<Vec<MaybeAssetLoader>>,
    extension_to_loader_index: RwLock<HashMap<String, usize>>,
    handle_to_path: Arc<RwLock<HashMap<HandleId, AssetPath<'static>>>>,
}

/// Loads assets from the filesystem in the background.
///
/// The asset server is the primary way of loading assets in bevy. It keeps track of the load state
/// of the assets it manages and can even reload them from the filesystem with
/// ```
/// # use bevy_asset::*;
/// # use bevy_app::*;
/// # use bevy_utils::Duration;
/// # let mut app = App::new();
/// // The asset plugin can be configured to watch for asset changes.
/// app.add_plugins(AssetPlugin {
///     watch_for_changes: ChangeWatcher::with_delay(Duration::from_millis(200)),
///     ..Default::default()
/// });
/// ```
///
/// The asset server is a _resource_, so in order to access it in a system you need a `Res`
/// accessor, like this:
///
/// ```rust,no_run
/// use bevy_asset::{AssetServer, Handle};
/// use bevy_ecs::prelude::{Commands, Res};
///
/// # #[derive(Debug, bevy_reflect::TypeUuid, bevy_reflect::TypePath)]
/// # #[uuid = "00000000-0000-0000-0000-000000000000"]
/// # struct Image;
///
/// fn my_system(mut commands: Commands, asset_server: Res<AssetServer>)
/// {
///     // Now you can do whatever you want with the asset server, such as loading an asset:
///     let asset_handle: Handle<Image> = asset_server.load("cool_picture.png");
/// }
/// ```
///
/// See the [`asset_loading`] example for more information.
///
/// [`asset_loading`]: https://github.com/bevyengine/bevy/tree/latest/examples/asset/asset_loading.rs
#[derive(Clone, Resource)]
pub struct AssetServer {
    pub(crate) server: Arc<AssetServerInternal>,
}

impl AssetServer {
    /// Creates a new asset server with the provided asset I/O.
    pub fn new<T: AssetIo>(source_io: T) -> Self {
        Self::with_boxed_io(Box::new(source_io))
    }

    /// Creates a new asset server with a boxed asset I/O.
    pub fn with_boxed_io(asset_io: Box<dyn AssetIo>) -> Self {
        AssetServer {
            server: Arc::new(AssetServerInternal {
                loaders: Default::default(),
                extension_to_loader_index: Default::default(),
                asset_sources: Default::default(),
                asset_ref_counter: Default::default(),
                handle_to_path: Default::default(),
                asset_lifecycles: Default::default(),
                asset_io,
            }),
        }
    }

    /// Returns the associated asset I/O.
    pub fn asset_io(&self) -> &dyn AssetIo {
        &*self.server.asset_io
    }

    pub(crate) fn register_asset_type<T: Asset>(&self) -> Assets<T> {
        if self
            .server
            .asset_lifecycles
            .write()
            .insert(T::TYPE_UUID, Box::<AssetLifecycleChannel<T>>::default())
            .is_some()
        {
            panic!("Error while registering new asset type: {:?} with UUID: {:?}. Another type with the same UUID is already registered. Can not register new asset type with the same UUID",
                std::any::type_name::<T>(), T::TYPE_UUID);
        }
        Assets::new(self.server.asset_ref_counter.channel.sender.clone())
    }

    /// Pre-register a loader that will later be added.
    ///
    /// Assets loaded with matching extensions will be blocked until the
    /// real loader is added.
    pub fn preregister_loader(&self, extensions: &[&str]) {
        let mut loaders = self.server.loaders.write();
        let loader_index = loaders.len();
        for extension in extensions {
            if self
                .server
                .extension_to_loader_index
                .write()
                .insert(extension.to_string(), loader_index)
                .is_some()
            {
                warn!("duplicate preregistration for `{extension}`, any assets loaded with the previous loader will never complete.");
            }
        }
        let (sender, receiver) = async_channel::bounded(1);
        loaders.push(MaybeAssetLoader::Pending { sender, receiver });
    }

    /// Adds the provided asset loader to the server.
    ///
    /// If `loader` has one or more supported extensions in conflict with loaders that came before
    /// it, it will replace them.
    pub fn add_loader<T>(&self, loader: T)
    where
        T: AssetLoader,
    {
        let mut loaders = self.server.loaders.write();
        let next_loader_index = loaders.len();
        let mut maybe_existing_loader_index = None;
        let mut loader_map = self.server.extension_to_loader_index.write();
        let mut maybe_sender = None;

        for extension in loader.extensions() {
            if let Some(&extension_index) = loader_map.get(*extension) {
                // replacing an existing entry
                match maybe_existing_loader_index {
                    None => {
                        match &loaders[extension_index] {
                            MaybeAssetLoader::Ready(_) => {
                                // replacing an existing loader, nothing special to do
                            }
                            MaybeAssetLoader::Pending { sender, .. } => {
                                // the loader was pre-registered, store the channel to notify pending assets
                                maybe_sender = Some(sender.clone());
                            }
                        }
                    }
                    Some(index) => {
                        // ensure the loader extensions are consistent
                        if index != extension_index {
                            warn!("inconsistent extensions between loader preregister_loader and add_loader, \
                                   loading `{extension}` assets will never complete.");
                        }
                    }
                }

                maybe_existing_loader_index = Some(extension_index);
            } else {
                loader_map.insert(extension.to_string(), next_loader_index);
            }
        }

        if let Some(existing_index) = maybe_existing_loader_index {
            loaders[existing_index] = MaybeAssetLoader::Ready(Arc::new(loader));
            if let Some(sender) = maybe_sender {
                // notify after replacing the loader
                let _ = sender.close();
            }
        } else {
            loaders.push(MaybeAssetLoader::Ready(Arc::new(loader)));
        }
    }

    /// Gets a strong handle for an asset with the provided id.
    pub fn get_handle<T: Asset, I: Into<HandleId>>(&self, id: I) -> Handle<T> {
        let sender = self.server.asset_ref_counter.channel.sender.clone();
        Handle::strong(id.into(), sender)
    }

    /// Gets an untyped strong handle for an asset with the provided id.
    pub fn get_handle_untyped<I: Into<HandleId>>(&self, id: I) -> HandleUntyped {
        let sender = self.server.asset_ref_counter.channel.sender.clone();
        HandleUntyped::strong(id.into(), sender)
    }

    fn get_asset_loader(&self, extension: &str) -> Result<MaybeAssetLoader, AssetServerError> {
        let index = {
            // scope map to drop lock as soon as possible
            let map = self.server.extension_to_loader_index.read();
            map.get(extension).copied()
        };
        index
            .map(|index| self.server.loaders.read()[index].clone())
            .ok_or_else(|| AssetServerError::MissingAssetLoader {
                extensions: vec![extension.to_string()],
            })
    }

    fn get_path_asset_loader<P: AsRef<Path>>(
        &self,
        path: P,
        include_pending: bool,
    ) -> Result<MaybeAssetLoader, AssetServerError> {
        let s = path
            .as_ref()
            .file_name()
            .ok_or(AssetServerError::MissingAssetLoader {
                extensions: Vec::new(),
            })?
            .to_str()
            .map(|s| s.to_lowercase())
            .ok_or(AssetServerError::MissingAssetLoader {
                extensions: Vec::new(),
            })?;

        let mut exts = Vec::new();
        let mut ext = s.as_str();
        while let Some(idx) = ext.find('.') {
            ext = &ext[idx + 1..];
            exts.push(ext);
            if let Ok(loader) = self.get_asset_loader(ext) {
                if include_pending || matches!(loader, MaybeAssetLoader::Ready(_)) {
                    return Ok(loader);
                }
            }
        }
        Err(AssetServerError::MissingAssetLoader {
            extensions: exts.into_iter().map(String::from).collect(),
        })
    }

    /// Gets the source path of an asset from the provided handle.
    pub fn get_handle_path<H: Into<HandleId>>(&self, handle: H) -> Option<AssetPath<'_>> {
        self.server
            .handle_to_path
            .read()
            .get(&handle.into())
            .cloned()
    }

    /// Gets the load state of an asset from the provided handle.
    pub fn get_load_state<H: Into<HandleId>>(&self, handle: H) -> LoadState {
        match handle.into() {
            HandleId::AssetPathId(id) => {
                let asset_sources = self.server.asset_sources.read();
                asset_sources
                    .get(&id.source_path_id())
                    .map_or(LoadState::NotLoaded, |info| info.load_state)
            }
            HandleId::Id(_, _) => LoadState::NotLoaded,
        }
    }

    /// Gets the overall load state of a group of assets from the provided handles.
    ///
    /// This method will only return [`LoadState::Loaded`] if all assets in the
    /// group were loaded successfully.
    pub fn get_group_load_state(&self, handles: impl IntoIterator<Item = HandleId>) -> LoadState {
        let mut load_state = LoadState::Loaded;
        for handle_id in handles {
            match handle_id {
                HandleId::AssetPathId(id) => match self.get_load_state(id) {
                    LoadState::Loaded => continue,
                    LoadState::Loading => {
                        load_state = LoadState::Loading;
                    }
                    LoadState::Failed => return LoadState::Failed,
                    LoadState::NotLoaded => return LoadState::NotLoaded,
                    LoadState::Unloaded => return LoadState::Unloaded,
                },
                HandleId::Id(_, _) => return LoadState::NotLoaded,
            }
        }

        load_state
    }

    /// Queues an [`Asset`] at the provided relative path for asynchronous loading.
    ///
    /// The absolute path to the asset is `"ROOT/ASSET_FOLDER_NAME/path"`. Its extension is then
    /// extracted to search for an [asset loader]. If an asset path contains multiple dots (e.g.
    /// `foo.bar.baz`), each level is considered a separate extension and the asset server will try
    /// to look for loaders of `bar.baz` and `baz` assets.
    ///
    /// By default the `ROOT` is the directory of the Application, but this can be overridden by
    /// setting the `"BEVY_ASSET_ROOT"` or `"CARGO_MANIFEST_DIR"` environment variable
    /// (see <https://doc.rust-lang.org/cargo/reference/environment-variables.html>)
    /// to another directory. When the application is run through Cargo, then
    /// `"CARGO_MANIFEST_DIR"` is automatically set to the root folder of your crate (workspace).
    ///
    /// The name of the asset folder is set inside the
    /// [`AssetPlugin`](crate::AssetPlugin). The default name is
    /// `"assets"`.
    ///
    /// The asset is loaded asynchronously, and will generally not be available by the time
    /// this calls returns. Use [`AssetServer::get_load_state`] to determine when the asset is
    /// effectively loaded and available in the [`Assets`] collection. The asset will always fail to
    /// load if the provided path doesn't contain an extension.
    ///
    /// [asset loader]: AssetLoader
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load<'a, T: Asset, P: Into<AssetPath<'a>>>(&self, path: P) -> Handle<T> {
        self.load_untyped(path).typed()
    }

    async fn load_async(
        &self,
        asset_path: AssetPath<'_>,
        force: bool,
    ) -> Result<AssetPathId, AssetServerError> {
        let asset_path_id: AssetPathId = asset_path.get_id();

        // load metadata and update source info. this is done in a scope to ensure we release the
        // locks before loading
        let version = {
            let mut asset_sources = self.server.asset_sources.write();
            let source_info = match asset_sources.entry(asset_path_id.source_path_id()) {
                Entry::Occupied(entry) => entry.into_mut(),
                Entry::Vacant(entry) => entry.insert(SourceInfo {
                    asset_types: Default::default(),
                    committed_assets: Default::default(),
                    load_state: LoadState::NotLoaded,
                    meta: None,
                    path: asset_path.path().to_owned(),
                    version: 0,
                }),
            };

            // if asset is already loaded or is loading, don't load again
            if !force
                && (source_info
                    .committed_assets
                    .contains(&asset_path_id.label_id())
                    || source_info.load_state == LoadState::Loading)
            {
                return Ok(asset_path_id);
            }

            source_info.load_state = LoadState::Loading;
            source_info.committed_assets.clear();
            source_info.version += 1;
            source_info.meta = None;
            source_info.version
        };

        let set_asset_failed = || {
            let mut asset_sources = self.server.asset_sources.write();
            let source_info = asset_sources
                .get_mut(&asset_path_id.source_path_id())
                .expect("`AssetSource` should exist at this point.");
            source_info.load_state = LoadState::Failed;
        };

        // get the according asset loader
        let mut maybe_asset_loader = self.get_path_asset_loader(asset_path.path(), true);

        // if it's still pending, block until notified and refetch the new asset loader
        if let Ok(MaybeAssetLoader::Pending { receiver, .. }) = maybe_asset_loader {
            let _ = receiver.recv().await;
            maybe_asset_loader = self.get_path_asset_loader(asset_path.path(), false);
        }

        let asset_loader = match maybe_asset_loader {
            Ok(MaybeAssetLoader::Ready(loader)) => loader,
            Err(err) => {
                set_asset_failed();
                return Err(err);
            }
            Ok(MaybeAssetLoader::Pending { .. }) => unreachable!(),
        };

        // load the asset bytes
        let bytes = match self.asset_io().load_path(asset_path.path()).await {
            Ok(bytes) => bytes,
            Err(err) => {
                set_asset_failed();
                return Err(AssetServerError::AssetIoError(err));
            }
        };

        // load the asset source using the corresponding AssetLoader
        let mut load_context = LoadContext::new(
            asset_path.path(),
            &self.server.asset_ref_counter.channel,
            self.asset_io(),
            version,
        );

        if let Err(err) = asset_loader
            .load(&bytes, &mut load_context)
            .await
            .map_err(AssetServerError::AssetLoaderError)
        {
            set_asset_failed();
            return Err(err);
        }

        // if version has changed since we loaded and grabbed a lock, return. there is a newer
        // version being loaded
        let mut asset_sources = self.server.asset_sources.write();
        let source_info = asset_sources
            .get_mut(&asset_path_id.source_path_id())
            .expect("`AssetSource` should exist at this point.");
        if version != source_info.version {
            return Ok(asset_path_id);
        }

        // if all assets have been committed already (aka there were 0), set state to "Loaded"
        if source_info.is_loaded() {
            source_info.load_state = LoadState::Loaded;
        }

        // reset relevant SourceInfo fields
        source_info.committed_assets.clear();
        // TODO: queue free old assets
        source_info.asset_types.clear();

        source_info.meta = Some(SourceMeta {
            assets: load_context.get_asset_metas(),
        });

        // load asset dependencies and prepare asset type hashmap
        for (label, loaded_asset) in &mut load_context.labeled_assets {
            let label_id = LabelId::from(label.as_ref().map(|label| label.as_str()));
            let type_uuid = loaded_asset.value.as_ref().unwrap().type_uuid();
            source_info.asset_types.insert(label_id, type_uuid);
            for dependency in &loaded_asset.dependencies {
                self.load_untracked(dependency.clone(), false);
            }
        }

        self.asset_io()
            .watch_path_for_changes(asset_path.path(), None)
            .unwrap();
        self.create_assets_in_load_context(&mut load_context);
        Ok(asset_path_id)
    }

    /// Queues the [`Asset`] at the provided path for loading and returns an untyped handle.
    ///
    /// See [`load`](AssetServer::load).
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load_untyped<'a, P: Into<AssetPath<'a>>>(&self, path: P) -> HandleUntyped {
        let handle_id = self.load_untracked(path.into(), false);
        self.get_handle_untyped(handle_id)
    }

    /// Force an [`Asset`] to be reloaded.
    ///
    /// This is useful for custom hot-reloading or for supporting `watch_for_changes`
    /// in custom [`AssetIo`] implementations.
    pub fn reload_asset<'a, P: Into<AssetPath<'a>>>(&self, path: P) {
        self.load_untracked(path.into(), true);
    }

    pub(crate) fn load_untracked(&self, asset_path: AssetPath<'_>, force: bool) -> HandleId {
        let server = self.clone();
        let owned_path = asset_path.to_owned();
        IoTaskPool::get()
            .spawn(async move {
                if let Err(err) = server.load_async(owned_path, force).await {
                    warn!("{}", err);
                }
            })
            .detach();

        let handle_id = asset_path.get_id().into();
        self.server
            .handle_to_path
            .write()
            .entry(handle_id)
            .or_insert_with(|| asset_path.to_owned());

        asset_path.into()
    }

    /// Loads assets from the specified folder recursively.
    ///
    /// # Errors
    ///
    /// - If the provided path is not a directory, it will fail with
    /// [`AssetServerError::AssetFolderNotADirectory`].
    /// - If something unexpected happened while loading an asset, other
    /// [`AssetServerError`]s may be returned.
    #[must_use = "not using the returned strong handles may result in the unexpected release of the assets"]
    pub fn load_folder<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<HandleUntyped>, AssetServerError> {
        let path = path.as_ref();
        if !self.asset_io().is_dir(path) {
            return Err(AssetServerError::AssetFolderNotADirectory(
                path.to_str().unwrap().to_string(),
            ));
        }

        let mut handles = Vec::new();
        for child_path in self.asset_io().read_directory(path.as_ref())? {
            if self.asset_io().is_dir(&child_path) {
                handles.extend(self.load_folder(&child_path)?);
            } else {
                if self.get_path_asset_loader(&child_path, true).is_err() {
                    continue;
                }
                let handle =
                    self.load_untyped(child_path.to_str().expect("Path should be a valid string."));
                handles.push(handle);
            }
        }

        Ok(handles)
    }

    /// Frees unused assets, unloading them from memory.
    pub fn free_unused_assets(&self) {
        let mut potential_frees = self.server.asset_ref_counter.mark_unused_assets.lock();

        if !potential_frees.is_empty() {
            let ref_counts = self.server.asset_ref_counter.ref_counts.read();
            let asset_sources = self.server.asset_sources.read();
            let asset_lifecycles = self.server.asset_lifecycles.read();
            for potential_free in potential_frees.drain(..) {
                if let Some(&0) = ref_counts.get(&potential_free) {
                    let type_uuid = match potential_free {
                        HandleId::Id(type_uuid, _) => Some(type_uuid),
                        HandleId::AssetPathId(id) => asset_sources
                            .get(&id.source_path_id())
                            .and_then(|source_info| source_info.get_asset_type(id.label_id())),
                    };

                    if let Some(type_uuid) = type_uuid {
                        if let Some(asset_lifecycle) = asset_lifecycles.get(&type_uuid) {
                            asset_lifecycle.free_asset(potential_free);
                        }
                    }
                }
            }
        }
    }

    /// Iterates through asset references and marks assets with no active handles as unused.
    pub fn mark_unused_assets(&self) {
        let receiver = &self.server.asset_ref_counter.channel.receiver;
        let mut ref_counts = self.server.asset_ref_counter.ref_counts.write();
        let mut potential_frees = None;
        loop {
            let ref_change = match receiver.try_recv() {
                Ok(ref_change) => ref_change,
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("RefChange channel disconnected."),
            };
            match ref_change {
                RefChange::Increment(handle_id) => *ref_counts.entry(handle_id).or_insert(0) += 1,
                RefChange::Decrement(handle_id) => {
                    let entry = ref_counts.entry(handle_id).or_insert(0);
                    *entry -= 1;
                    if *entry == 0 {
                        potential_frees
                            .get_or_insert_with(|| {
                                self.server.asset_ref_counter.mark_unused_assets.lock()
                            })
                            .push(handle_id);
                    }
                }
            }
        }
    }

    fn create_assets_in_load_context(&self, load_context: &mut LoadContext) {
        let asset_lifecycles = self.server.asset_lifecycles.read();
        for (label, asset) in &mut load_context.labeled_assets {
            let asset_value = asset
                .value
                .take()
                .expect("Asset should exist at this point.");
            if let Some(asset_lifecycle) = asset_lifecycles.get(&asset_value.type_uuid()) {
                let asset_path =
                    AssetPath::new_ref(load_context.path, label.as_ref().map(|l| l.as_str()));
                asset_lifecycle.create_asset(asset_path.into(), asset_value, load_context.version);
            } else {
                panic!(
                    "Failed to find AssetLifecycle for label '{:?}', which has an asset type {} (UUID {:?}). \
                        Are you sure this asset type has been added to your app builder?",
                    label,
                    asset_value.type_name(),
                    asset_value.type_uuid(),
                );
            }
        }
    }

    // Note: this takes a `ResMut<Assets<T>>` to ensure change detection does not get
    // triggered unless the `Assets` collection is actually updated.
    pub(crate) fn update_asset_storage<T: Asset>(&self, mut assets: ResMut<Assets<T>>) {
        let asset_lifecycles = self.server.asset_lifecycles.read();
        let asset_lifecycle = asset_lifecycles.get(&T::TYPE_UUID).unwrap();
        let mut asset_sources_guard = None;
        let channel = asset_lifecycle
            .downcast_ref::<AssetLifecycleChannel<T>>()
            .unwrap();

        loop {
            match channel.receiver.try_recv() {
                Ok(AssetLifecycleEvent::Create(result)) => {
                    // update SourceInfo if this asset was loaded from an AssetPath
                    if let HandleId::AssetPathId(id) = result.id {
                        let asset_sources = asset_sources_guard
                            .get_or_insert_with(|| self.server.asset_sources.write());
                        if let Some(source_info) = asset_sources.get_mut(&id.source_path_id()) {
                            if source_info.version == result.version {
                                source_info.committed_assets.insert(id.label_id());
                                if source_info.is_loaded() {
                                    source_info.load_state = LoadState::Loaded;
                                }
                            }
                        }
                    }

                    assets.set_untracked(result.id, *result.asset);
                }
                Ok(AssetLifecycleEvent::Free(handle_id)) => {
                    if let HandleId::AssetPathId(id) = handle_id {
                        let asset_sources = asset_sources_guard
                            .get_or_insert_with(|| self.server.asset_sources.write());
                        if let Some(source_info) = asset_sources.get_mut(&id.source_path_id()) {
                            source_info.committed_assets.remove(&id.label_id());
                            source_info.load_state = LoadState::Unloaded;
                        }
                    }
                    assets.remove(handle_id);
                }
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => panic!("AssetChannel disconnected."),
            }
        }
    }
}

fn free_unused_assets_system_impl(asset_server: &AssetServer) {
    asset_server.free_unused_assets();
    asset_server.mark_unused_assets();
}

/// A system for freeing assets that have no active handles.
pub fn free_unused_assets_system(asset_server: Res<AssetServer>) {
    free_unused_assets_system_impl(&asset_server);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{loader::LoadedAsset, update_asset_storage_system};
    use bevy_app::{App, Update};
    use bevy_ecs::prelude::*;
    use bevy_reflect::{TypePath, TypeUuid};
    use bevy_utils::BoxedFuture;

    #[derive(Debug, TypeUuid, TypePath)]
    #[uuid = "a5189b72-0572-4290-a2e0-96f73a491c44"]
    struct PngAsset;

    struct FakePngLoader;
    impl AssetLoader for FakePngLoader {
        fn load<'a>(
            &'a self,
            _: &'a [u8],
            ctx: &'a mut LoadContext,
        ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
            ctx.set_default_asset(LoadedAsset::new(PngAsset));
            Box::pin(async move { Ok(()) })
        }

        fn extensions(&self) -> &[&str] {
            &["png"]
        }
    }

    struct FailingLoader;
    impl AssetLoader for FailingLoader {
        fn load<'a>(
            &'a self,
            _: &'a [u8],
            _: &'a mut LoadContext,
        ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
            Box::pin(async { anyhow::bail!("failed") })
        }

        fn extensions(&self) -> &[&str] {
            &["fail"]
        }
    }

    struct FakeMultipleDotLoader;
    impl AssetLoader for FakeMultipleDotLoader {
        fn load<'a>(
            &'a self,
            _: &'a [u8],
            _: &'a mut LoadContext,
        ) -> BoxedFuture<'a, Result<(), anyhow::Error>> {
            Box::pin(async move { Ok(()) })
        }

        fn extensions(&self) -> &[&str] {
            &["test.png"]
        }
    }

    fn setup(asset_path: impl AsRef<Path>) -> AssetServer {
        use crate::FileAssetIo;
        IoTaskPool::init(Default::default);
        AssetServer::new(FileAssetIo::new(asset_path, &None))
    }

    #[test]
    fn extensions() {
        let asset_server = setup(".");
        asset_server.add_loader(FakePngLoader);

        let Ok(MaybeAssetLoader::Ready(t)) = asset_server.get_path_asset_loader("test.png", true) else {
            panic!();
        };

        assert_eq!(t.extensions()[0], "png");
    }

    #[test]
    fn case_insensitive_extensions() {
        let asset_server = setup(".");
        asset_server.add_loader(FakePngLoader);

        let Ok(MaybeAssetLoader::Ready(t)) = asset_server.get_path_asset_loader("test.PNG", true) else {
            panic!();
        };
        assert_eq!(t.extensions()[0], "png");
    }

    #[test]
    fn no_loader() {
        let asset_server = setup(".");
        let t = asset_server.get_path_asset_loader("test.pong", true);
        assert!(t.is_err());
    }

    #[test]
    fn multiple_extensions_no_loader() {
        let asset_server = setup(".");

        assert!(
            match asset_server.get_path_asset_loader("test.v1.2.3.pong", true) {
                Err(AssetServerError::MissingAssetLoader { extensions }) =>
                    extensions == vec!["v1.2.3.pong", "2.3.pong", "3.pong", "pong"],
                _ => false,
            }
        );
    }

    #[test]
    fn missing_asset_loader_error_messages() {
        assert_eq!(
            AssetServerError::MissingAssetLoader { extensions: vec![] }.to_string(),
            "no `AssetLoader` found"
        );
        assert_eq!(
            AssetServerError::MissingAssetLoader {
                extensions: vec!["png".into()]
            }
            .to_string(),
            "no `AssetLoader` found for the following extension: png"
        );
        assert_eq!(
            AssetServerError::MissingAssetLoader {
                extensions: vec!["1.2.png".into(), "2.png".into(), "png".into()]
            }
            .to_string(),
            "no `AssetLoader` found for the following extensions: 1.2.png, 2.png, png"
        );
    }

    #[test]
    fn filename_with_dots() {
        let asset_server = setup(".");
        asset_server.add_loader(FakePngLoader);

        let Ok(MaybeAssetLoader::Ready(t)) = asset_server.get_path_asset_loader("test-v1.2.3.png", true) else {
            panic!();
        };
        assert_eq!(t.extensions()[0], "png");
    }

    #[test]
    fn multiple_extensions() {
        let asset_server = setup(".");
        asset_server.add_loader(FakeMultipleDotLoader);

        let Ok(MaybeAssetLoader::Ready(t)) = asset_server.get_path_asset_loader("test.test.png", true) else {
            panic!();
        };
        assert_eq!(t.extensions()[0], "test.png");
    }

    fn create_dir_and_file(file: impl AsRef<Path>) -> tempfile::TempDir {
        let asset_dir = tempfile::tempdir().unwrap();
        std::fs::write(asset_dir.path().join(file), []).unwrap();
        asset_dir
    }

    #[test]
    fn test_missing_loader() {
        let dir = create_dir_and_file("file.not-a-real-extension");
        let asset_server = setup(dir.path());

        let path: AssetPath = "file.not-a-real-extension".into();
        let handle = asset_server.get_handle_untyped(path.get_id());

        let err = futures_lite::future::block_on(asset_server.load_async(path.clone(), true))
            .unwrap_err();
        assert!(match err {
            AssetServerError::MissingAssetLoader { extensions } => {
                extensions == ["not-a-real-extension"]
            }
            _ => false,
        });

        assert_eq!(asset_server.get_load_state(handle), LoadState::Failed);
    }

    #[test]
    fn test_invalid_asset_path() {
        let asset_server = setup(".");
        asset_server.add_loader(FakePngLoader);

        let path: AssetPath = "an/invalid/path.png".into();
        let handle = asset_server.get_handle_untyped(path.get_id());

        let err = futures_lite::future::block_on(asset_server.load_async(path.clone(), true))
            .unwrap_err();
        assert!(matches!(err, AssetServerError::AssetIoError(_)));

        assert_eq!(asset_server.get_load_state(handle), LoadState::Failed);
    }

    #[test]
    fn test_failing_loader() {
        let dir = create_dir_and_file("fake.fail");
        let asset_server = setup(dir.path());
        asset_server.add_loader(FailingLoader);

        let path: AssetPath = "fake.fail".into();
        let handle = asset_server.get_handle_untyped(path.get_id());

        let err = futures_lite::future::block_on(asset_server.load_async(path.clone(), true))
            .unwrap_err();
        assert!(matches!(err, AssetServerError::AssetLoaderError(_)));

        assert_eq!(asset_server.get_load_state(handle), LoadState::Failed);
    }

    #[test]
    fn test_asset_lifecycle() {
        let dir = create_dir_and_file("fake.png");
        let asset_server = setup(dir.path());
        asset_server.add_loader(FakePngLoader);
        let assets = asset_server.register_asset_type::<PngAsset>();

        #[derive(SystemSet, Clone, Hash, Debug, PartialEq, Eq)]
        struct FreeUnusedAssets;
        let mut app = App::new();
        app.insert_resource(assets);
        app.insert_resource(asset_server);
        app.add_systems(
            Update,
            (
                free_unused_assets_system.in_set(FreeUnusedAssets),
                update_asset_storage_system::<PngAsset>.after(FreeUnusedAssets),
            ),
        );

        fn load_asset(path: AssetPath, world: &World) -> HandleUntyped {
            let asset_server = world.resource::<AssetServer>();
            let id = futures_lite::future::block_on(asset_server.load_async(path.clone(), true))
                .unwrap();
            asset_server.get_handle_untyped(id)
        }

        fn get_asset<'world>(
            id: &Handle<PngAsset>,
            world: &'world World,
        ) -> Option<&'world PngAsset> {
            world.resource::<Assets<PngAsset>>().get(id)
        }

        fn get_load_state(id: impl Into<HandleId>, world: &World) -> LoadState {
            world.resource::<AssetServer>().get_load_state(id.into())
        }

        // ---
        // Start of the actual lifecycle test
        // ---

        let path: AssetPath = "fake.png".into();
        assert_eq!(
            LoadState::NotLoaded,
            get_load_state(path.get_id(), &app.world)
        );

        // load the asset
        let handle = load_asset(path.clone(), &app.world).typed();
        let weak_handle = handle.clone_weak();

        // asset is loading
        assert_eq!(LoadState::Loading, get_load_state(&handle, &app.world));

        app.update();
        // asset should exist and be loaded at this point
        assert_eq!(LoadState::Loaded, get_load_state(&handle, &app.world));
        assert!(get_asset(&handle, &app.world).is_some());

        // after dropping the handle, next call to `tick` will prepare the assets for removal.
        drop(handle);
        app.update();
        assert_eq!(LoadState::Loaded, get_load_state(&weak_handle, &app.world));
        assert!(get_asset(&weak_handle, &app.world).is_some());

        // second call to tick will actually remove the asset.
        app.update();
        assert_eq!(
            LoadState::Unloaded,
            get_load_state(&weak_handle, &app.world)
        );
        assert!(get_asset(&weak_handle, &app.world).is_none());

        // finally, reload the asset
        let handle = load_asset(path.clone(), &app.world).typed();
        assert_eq!(LoadState::Loading, get_load_state(&handle, &app.world));
        app.update();
        assert_eq!(LoadState::Loaded, get_load_state(&handle, &app.world));
        assert!(get_asset(&handle, &app.world).is_some());
    }

    #[test]
    fn test_get_handle_path() {
        const PATH: &str = "path/file.png";

        // valid handle
        let server = setup(".");
        let handle = server.load_untyped(PATH);
        let handle_path = server.get_handle_path(&handle).unwrap();

        assert_eq!(handle_path.path(), Path::new(PATH));
        assert!(handle_path.label().is_none());

        let handle_id: HandleId = handle.into();
        let path_id: HandleId = handle_path.get_id().into();
        assert_eq!(handle_id, path_id);

        // invalid handle (not loaded through server)
        let mut assets = server.register_asset_type::<PngAsset>();
        let handle = assets.add(PngAsset);
        assert!(server.get_handle_path(&handle).is_none());

        // invalid HandleId
        let invalid_id = HandleId::new(Uuid::new_v4(), 42);
        assert!(server.get_handle_path(invalid_id).is_none());

        // invalid AssetPath
        let invalid_path = AssetPath::new("some/path.ext".into(), None);
        assert!(server.get_handle_path(invalid_path).is_none());
    }
}
