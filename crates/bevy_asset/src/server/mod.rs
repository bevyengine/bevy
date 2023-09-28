mod info;

use crate::{
    folder::LoadedFolder,
    io::{AssetReader, AssetReaderError, AssetSourceEvent, AssetWatcher, Reader},
    loader::{AssetLoader, AssetLoaderError, ErasedAssetLoader, LoadContext, LoadedAsset},
    meta::{
        loader_settings_meta_transform, AssetActionMinimal, AssetMetaDyn, AssetMetaMinimal,
        MetaTransform, Settings,
    },
    path::AssetPath,
    Asset, AssetEvent, AssetHandleProvider, AssetId, Assets, DeserializeMetaError,
    ErasedLoadedAsset, Handle, UntypedAssetId, UntypedHandle,
};
use bevy_ecs::prelude::*;
use bevy_log::{error, info, warn};
use bevy_tasks::IoTaskPool;
use bevy_utils::{HashMap, HashSet};
use crossbeam_channel::{Receiver, Sender};
use futures_lite::StreamExt;
use info::*;
use parking_lot::RwLock;
use std::{any::TypeId, path::Path, sync::Arc};
use thiserror::Error;

/// Loads and tracks the state of [`Asset`] values from a configured [`AssetReader`]. This can be used to kick off new asset loads and
/// retrieve their current load states.
///
/// The general process to load an asset is:
/// 1. Initialize a new [`Asset`] type with the [`AssetServer`] via [`AssetApp::init_asset`], which will internally call [`AssetServer::register_asset`]
/// and set up related ECS [`Assets`] storage and systems.
/// 2. Register one or more [`AssetLoader`]s for that asset with [`AssetApp::init_asset_loader`]  
/// 3. Add the asset to your asset folder (defaults to `assets`).
/// 4. Call [`AssetServer::load`] with a path to your asset.
///
/// [`AssetServer`] can be cloned. It is backed by an [`Arc`] so clones will share state. Clones can be freely used in parallel.
///
/// [`AssetApp::init_asset`]: crate::AssetApp::init_asset
/// [`AssetApp::init_asset_loader`]: crate::AssetApp::init_asset_loader
#[derive(Resource, Clone)]
pub struct AssetServer {
    pub(crate) data: Arc<AssetServerData>,
}

/// Internal data used by [`AssetServer`]. This is intended to be used from within an [`Arc`].
pub(crate) struct AssetServerData {
    pub(crate) infos: RwLock<AssetInfos>,
    pub(crate) loaders: Arc<RwLock<AssetLoaders>>,
    asset_event_sender: Sender<InternalAssetEvent>,
    asset_event_receiver: Receiver<InternalAssetEvent>,
    source_event_receiver: Receiver<AssetSourceEvent>,
    reader: Box<dyn AssetReader>,
    _watcher: Option<Box<dyn AssetWatcher>>,
}

impl AssetServer {
    /// Create a new instance of [`AssetServer`]. If `watch_for_changes` is true, the [`AssetReader`] storage will watch for changes to
    /// asset sources and hot-reload them.
    pub fn new(reader: Box<dyn AssetReader>, watch_for_changes: bool) -> Self {
        Self::new_with_loaders(reader, Default::default(), watch_for_changes)
    }

    pub(crate) fn new_with_loaders(
        reader: Box<dyn AssetReader>,
        loaders: Arc<RwLock<AssetLoaders>>,
        watch_for_changes: bool,
    ) -> Self {
        let (asset_event_sender, asset_event_receiver) = crossbeam_channel::unbounded();
        let (source_event_sender, source_event_receiver) = crossbeam_channel::unbounded();
        let mut infos = AssetInfos::default();
        let watcher = if watch_for_changes {
            infos.watching_for_changes = true;
            let watcher = reader.watch_for_changes(source_event_sender);
            if watcher.is_none() {
                error!("{}", CANNOT_WATCH_ERROR_MESSAGE);
            }
            watcher
        } else {
            None
        };
        Self {
            data: Arc::new(AssetServerData {
                reader,
                _watcher: watcher,
                asset_event_sender,
                asset_event_receiver,
                source_event_receiver,
                loaders,
                infos: RwLock::new(infos),
            }),
        }
    }

    /// Returns the primary [`AssetReader`].
    pub fn reader(&self) -> &dyn AssetReader {
        &*self.data.reader
    }

    /// Registers a new [`AssetLoader`]. [`AssetLoader`]s must be registered before they can be used.
    pub fn register_loader<L: AssetLoader>(&self, loader: L) {
        let mut loaders = self.data.loaders.write();
        let type_name = std::any::type_name::<L>();
        let loader = Arc::new(loader);
        let (loader_index, is_new) =
            if let Some(index) = loaders.preregistered_loaders.remove(type_name) {
                (index, false)
            } else {
                (loaders.values.len(), true)
            };
        for extension in loader.extensions() {
            loaders
                .extension_to_index
                .insert(extension.to_string(), loader_index);
        }

        if is_new {
            loaders.type_name_to_index.insert(type_name, loader_index);
            loaders.values.push(MaybeAssetLoader::Ready(loader));
        } else {
            let maybe_loader = std::mem::replace(
                &mut loaders.values[loader_index],
                MaybeAssetLoader::Ready(loader.clone()),
            );
            match maybe_loader {
                MaybeAssetLoader::Ready(_) => unreachable!(),
                MaybeAssetLoader::Pending { sender, .. } => {
                    IoTaskPool::get()
                        .spawn(async move {
                            let _ = sender.broadcast(loader).await;
                        })
                        .detach();
                }
            }
        }
    }

    /// Registers a new [`Asset`] type. [`Asset`] types must be registered before assets of that type can be loaded.
    pub fn register_asset<A: Asset>(&self, assets: &Assets<A>) {
        self.register_handle_provider(assets.get_handle_provider());
        fn sender<A: Asset>(world: &mut World, id: UntypedAssetId) {
            world
                .resource_mut::<Events<AssetEvent<A>>>()
                .send(AssetEvent::LoadedWithDependencies { id: id.typed() });
        }
        self.data
            .infos
            .write()
            .dependency_loaded_event_sender
            .insert(TypeId::of::<A>(), sender::<A>);
    }

    pub(crate) fn register_handle_provider(&self, handle_provider: AssetHandleProvider) {
        let mut infos = self.data.infos.write();
        infos
            .handle_providers
            .insert(handle_provider.type_id, handle_provider);
    }

    /// Returns the registered [`AssetLoader`] associated with the given extension, if it exists.
    pub async fn get_asset_loader_with_extension(
        &self,
        extension: &str,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForExtensionError> {
        let loader = {
            let loaders = self.data.loaders.read();
            let index = *loaders.extension_to_index.get(extension).ok_or_else(|| {
                MissingAssetLoaderForExtensionError {
                    extensions: vec![extension.to_string()],
                }
            })?;
            loaders.values[index].clone()
        };

        match loader {
            MaybeAssetLoader::Ready(loader) => Ok(loader),
            MaybeAssetLoader::Pending { mut receiver, .. } => Ok(receiver.recv().await.unwrap()),
        }
    }

    /// Returns the registered [`AssetLoader`] associated with the given [`std::any::type_name`], if it exists.
    pub async fn get_asset_loader_with_type_name(
        &self,
        type_name: &str,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForTypeNameError> {
        let loader = {
            let loaders = self.data.loaders.read();
            let index = *loaders.type_name_to_index.get(type_name).ok_or_else(|| {
                MissingAssetLoaderForTypeNameError {
                    type_name: type_name.to_string(),
                }
            })?;

            loaders.values[index].clone()
        };
        match loader {
            MaybeAssetLoader::Ready(loader) => Ok(loader),
            MaybeAssetLoader::Pending { mut receiver, .. } => Ok(receiver.recv().await.unwrap()),
        }
    }

    /// Retrieves the default [`AssetLoader`] for the given path, if one can be found.
    pub async fn get_path_asset_loader<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForExtensionError> {
        let path = path.into();
        let full_extension =
            path.get_full_extension()
                .ok_or(MissingAssetLoaderForExtensionError {
                    extensions: Vec::new(),
                })?;
        if let Ok(loader) = self.get_asset_loader_with_extension(&full_extension).await {
            return Ok(loader);
        }
        for extension in AssetPath::iter_secondary_extensions(&full_extension) {
            if let Ok(loader) = self.get_asset_loader_with_extension(extension).await {
                return Ok(loader);
            }
        }
        let mut extensions = vec![full_extension.clone()];
        extensions
            .extend(AssetPath::iter_secondary_extensions(&full_extension).map(|e| e.to_string()));
        Err(MissingAssetLoaderForExtensionError { extensions })
    }

    /// Begins loading an [`Asset`] of type `A` stored at `path`. This will not block on the asset load. Instead,
    /// it returns a "strong" [`Handle`]. When the [`Asset`] is loaded (and enters [`LoadState::Loaded`]), it will be added to the
    /// associated [`Assets`] resource.
    ///
    /// You can check the asset's load state by reading [`AssetEvent`] events, calling [`AssetServer::load_state`], or checking
    /// the [`Assets`] storage to see if the [`Asset`] exists yet.
    ///
    /// The asset load will fail and an error will be printed to the logs if the asset stored at `path` is not of type `A`.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A> {
        self.load_with_meta_transform(path, None)
    }

    /// Begins loading an [`Asset`] of type `A` stored at `path`. The given `settings` function will override the asset's
    /// [`AssetLoader`] settings. The type `S` _must_ match the configured [`AssetLoader::Settings`] or `settings` changes
    /// will be ignored and an error will be printed to the log.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load_with_settings<'a, A: Asset, S: Settings>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A> {
        self.load_with_meta_transform(path, Some(loader_settings_meta_transform(settings)))
    }

    fn load_with_meta_transform<'a, A: Asset>(
        &self,
        path: impl Into<AssetPath<'a>>,
        meta_transform: Option<MetaTransform>,
    ) -> Handle<A> {
        let mut path = path.into().into_owned();
        let (handle, should_load) = self.data.infos.write().get_or_create_path_handle::<A>(
            path.clone(),
            HandleLoadingMode::Request,
            meta_transform,
        );

        if should_load {
            let mut owned_handle = Some(handle.clone().untyped());
            let server = self.clone();
            IoTaskPool::get()
                .spawn(async move {
                    if path.take_label().is_some() {
                        owned_handle = None;
                    }
                    if let Err(err) = server.load_internal(owned_handle, path, false, None).await {
                        error!("{}", err);
                    }
                })
                .detach();
        }

        handle
    }

    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub(crate) async fn load_untyped_async<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
    ) -> Result<UntypedHandle, AssetLoadError> {
        let path: AssetPath = path.into();
        self.load_internal(None, path, false, None).await
    }

    async fn load_internal<'a>(
        &self,
        input_handle: Option<UntypedHandle>,
        path: AssetPath<'a>,
        force: bool,
        meta_transform: Option<MetaTransform>,
    ) -> Result<UntypedHandle, AssetLoadError> {
        let mut path = path.into_owned();
        let path_clone = path.clone();
        let (mut meta, loader, mut reader) = self
            .get_meta_loader_and_reader(&path_clone)
            .await
            .map_err(|e| {
                // if there was an input handle, a "load" operation has already started, so we must produce a "failure" event, if
                // we cannot find the meta and loader
                if let Some(handle) = &input_handle {
                    self.send_asset_event(InternalAssetEvent::Failed { id: handle.id() });
                }
                e
            })?;

        let has_label = path.label().is_some();

        let (handle, should_load) = match input_handle {
            Some(handle) => {
                if !has_label && handle.type_id() != loader.asset_type_id() {
                    return Err(AssetLoadError::RequestedHandleTypeMismatch {
                        path: path.into_owned(),
                        requested: handle.type_id(),
                        actual_asset_name: loader.asset_type_name(),
                        loader_name: loader.type_name(),
                    });
                }
                // if a handle was passed in, the "should load" check was already done
                (handle, true)
            }
            None => {
                let mut infos = self.data.infos.write();
                infos.get_or_create_path_handle_untyped(
                    path.clone(),
                    loader.asset_type_id(),
                    loader.asset_type_name(),
                    HandleLoadingMode::Request,
                    meta_transform,
                )
            }
        };

        if !should_load && !force {
            return Ok(handle);
        }
        let base_asset_id = if has_label {
            path.remove_label();
            // If the path has a label, the current id does not match the asset root type.
            // We need to get the actual asset id
            let mut infos = self.data.infos.write();
            let (actual_handle, _) = infos.get_or_create_path_handle_untyped(
                path.clone(),
                loader.asset_type_id(),
                loader.asset_type_name(),
                // ignore current load state ... we kicked off this sub asset load because it needed to be loaded but
                // does not currently exist
                HandleLoadingMode::Force,
                None,
            );
            actual_handle.id()
        } else {
            handle.id()
        };

        if let Some(meta_transform) = handle.meta_transform() {
            (*meta_transform)(&mut *meta);
        }

        match self
            .load_with_meta_loader_and_reader(&path, meta, &*loader, &mut *reader, true, false)
            .await
        {
            Ok(mut loaded_asset) => {
                for (_, labeled_asset) in loaded_asset.labeled_assets.drain() {
                    self.send_asset_event(InternalAssetEvent::Loaded {
                        id: labeled_asset.handle.id(),
                        loaded_asset: labeled_asset.asset,
                    });
                }
                self.send_asset_event(InternalAssetEvent::Loaded {
                    id: base_asset_id,
                    loaded_asset,
                });
                Ok(handle)
            }
            Err(err) => {
                self.send_asset_event(InternalAssetEvent::Failed { id: base_asset_id });
                Err(err)
            }
        }
    }

    /// Kicks off a reload of the asset stored at the given path. This will only reload the asset if it currently loaded.
    pub fn reload<'a>(&self, path: impl Into<AssetPath<'a>>) {
        let server = self.clone();
        let path = path.into().into_owned();
        IoTaskPool::get()
            .spawn(async move {
                if server.data.infos.read().is_path_alive(&path) {
                    info!("Reloading {path} because it has changed");
                    if let Err(err) = server.load_internal(None, path, true, None).await {
                        error!("{}", err);
                    }
                }
            })
            .detach();
    }

    /// Queues a new asset to be tracked by the [`AssetServer`] and returns a [`Handle`] to it. This can be used to track
    /// dependencies of assets created at runtime.
    ///
    /// After the asset has been fully loaded by the [`AssetServer`], it will show up in the relevant [`Assets`] storage.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn add<A: Asset>(&self, asset: A) -> Handle<A> {
        self.load_asset(LoadedAsset::new_with_dependencies(asset, None))
    }

    pub(crate) fn load_asset<A: Asset>(&self, asset: impl Into<LoadedAsset<A>>) -> Handle<A> {
        let loaded_asset: LoadedAsset<A> = asset.into();
        let erased_loaded_asset: ErasedLoadedAsset = loaded_asset.into();
        self.load_asset_untyped(None, erased_loaded_asset)
            .typed_debug_checked()
    }

    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub(crate) fn load_asset_untyped(
        &self,
        path: Option<AssetPath<'static>>,
        asset: impl Into<ErasedLoadedAsset>,
    ) -> UntypedHandle {
        let loaded_asset = asset.into();
        let handle = if let Some(path) = path {
            let (handle, _) = self.data.infos.write().get_or_create_path_handle_untyped(
                path,
                loaded_asset.asset_type_id(),
                loaded_asset.asset_type_name(),
                HandleLoadingMode::NotLoading,
                None,
            );
            handle
        } else {
            self.data.infos.write().create_loading_handle_untyped(
                loaded_asset.asset_type_id(),
                loaded_asset.asset_type_name(),
            )
        };
        self.send_asset_event(InternalAssetEvent::Loaded {
            id: handle.id(),
            loaded_asset,
        });
        handle
    }

    /// Loads all assets from the specified folder recursively. The [`LoadedFolder`] asset (when it loads) will
    /// contain handles to all assets in the folder. You can wait for all assets to load by checking the [`LoadedFolder`]'s
    /// [`RecursiveDependencyLoadState`].
    #[must_use = "not using the returned strong handle may result in the unexpected release of the assets"]
    pub fn load_folder(&self, path: impl AsRef<Path>) -> Handle<LoadedFolder> {
        let handle = {
            let mut infos = self.data.infos.write();
            infos.create_loading_handle::<LoadedFolder>()
        };
        let id = handle.id().untyped();

        fn load_folder<'a>(
            path: &'a Path,
            server: &'a AssetServer,
            handles: &'a mut Vec<UntypedHandle>,
        ) -> bevy_utils::BoxedFuture<'a, Result<(), AssetLoadError>> {
            Box::pin(async move {
                let is_dir = server.reader().is_directory(path).await?;
                if is_dir {
                    let mut path_stream = server.reader().read_directory(path.as_ref()).await?;
                    while let Some(child_path) = path_stream.next().await {
                        if server.reader().is_directory(&child_path).await? {
                            load_folder(&child_path, server, handles).await?;
                        } else {
                            let path = child_path.to_str().expect("Path should be a valid string.");
                            match server.load_untyped_async(AssetPath::new(path)).await {
                                Ok(handle) => handles.push(handle),
                                // skip assets that cannot be loaded
                                Err(
                                    AssetLoadError::MissingAssetLoaderForTypeName(_)
                                    | AssetLoadError::MissingAssetLoaderForExtension(_),
                                ) => {}
                                Err(err) => return Err(err),
                            }
                        }
                    }
                }
                Ok(())
            })
        }

        let server = self.clone();
        let owned_path = path.as_ref().to_owned();
        IoTaskPool::get()
            .spawn(async move {
                let mut handles = Vec::new();
                match load_folder(&owned_path, &server, &mut handles).await {
                    Ok(_) => server.send_asset_event(InternalAssetEvent::Loaded {
                        id,
                        loaded_asset: LoadedAsset::new_with_dependencies(
                            LoadedFolder { handles },
                            None,
                        )
                        .into(),
                    }),
                    Err(_) => server.send_asset_event(InternalAssetEvent::Failed { id }),
                }
            })
            .detach();

        handle
    }

    fn send_asset_event(&self, event: InternalAssetEvent) {
        self.data.asset_event_sender.send(event).unwrap();
    }

    /// Retrieves all loads states for the given asset id.
    pub fn get_load_states(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<(LoadState, DependencyLoadState, RecursiveDependencyLoadState)> {
        self.data
            .infos
            .read()
            .get(id.into())
            .map(|i| (i.load_state, i.dep_load_state, i.rec_dep_load_state))
    }

    /// Retrieves the main [`LoadState`] of a given asset `id`.
    pub fn get_load_state(&self, id: impl Into<UntypedAssetId>) -> Option<LoadState> {
        self.data.infos.read().get(id.into()).map(|i| i.load_state)
    }

    /// Retrieves the [`RecursiveDependencyLoadState`] of a given asset `id`.
    pub fn get_recursive_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<RecursiveDependencyLoadState> {
        self.data
            .infos
            .read()
            .get(id.into())
            .map(|i| i.rec_dep_load_state)
    }

    /// Retrieves the main [`LoadState`] of a given asset `id`.
    pub fn load_state(&self, id: impl Into<UntypedAssetId>) -> LoadState {
        self.get_load_state(id).unwrap_or(LoadState::NotLoaded)
    }

    /// Retrieves the  [`RecursiveDependencyLoadState`] of a given asset `id`.
    pub fn recursive_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> RecursiveDependencyLoadState {
        self.get_recursive_dependency_load_state(id)
            .unwrap_or(RecursiveDependencyLoadState::NotLoaded)
    }

    /// Returns an active handle for the given path, if the asset at the given path has already started loading,
    /// or is still "alive".
    pub fn get_handle<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Option<Handle<A>> {
        self.get_handle_untyped(path)
            .map(|h| h.typed_debug_checked())
    }

    pub fn get_id_handle<A: Asset>(&self, id: AssetId<A>) -> Option<Handle<A>> {
        self.get_id_handle_untyped(id.untyped()).map(|h| h.typed())
    }

    pub fn get_id_handle_untyped(&self, id: UntypedAssetId) -> Option<UntypedHandle> {
        self.data.infos.read().get_id_handle(id)
    }

    /// Returns an active untyped handle for the given path, if the asset at the given path has already started loading,
    /// or is still "alive".
    pub fn get_handle_untyped<'a>(&self, path: impl Into<AssetPath<'a>>) -> Option<UntypedHandle> {
        let infos = self.data.infos.read();
        let path = path.into();
        infos.get_path_handle(path)
    }

    /// Returns the path for the given `id`, if it has one.
    pub fn get_path(&self, id: impl Into<UntypedAssetId>) -> Option<AssetPath> {
        let infos = self.data.infos.read();
        let info = infos.get(id.into())?;
        Some(info.path.as_ref()?.clone())
    }

    /// Pre-register a loader that will later be added.
    ///
    /// Assets loaded with matching extensions will be blocked until the
    /// real loader is added.
    pub fn preregister_loader<L: AssetLoader>(&self, extensions: &[&str]) {
        let mut loaders = self.data.loaders.write();
        let loader_index = loaders.values.len();
        let type_name = std::any::type_name::<L>();
        loaders
            .preregistered_loaders
            .insert(type_name, loader_index);
        loaders.type_name_to_index.insert(type_name, loader_index);
        for extension in extensions {
            if loaders
                .extension_to_index
                .insert(extension.to_string(), loader_index)
                .is_some()
            {
                warn!("duplicate preregistration for `{extension}`, any assets loaded with the previous loader will never complete.");
            }
        }
        let (mut sender, receiver) = async_broadcast::broadcast(1);
        sender.set_overflow(true);
        loaders
            .values
            .push(MaybeAssetLoader::Pending { sender, receiver });
    }

    /// Retrieve a handle for the given path. This will create a handle (and [`AssetInfo`]) if it does not exist
    pub(crate) fn get_or_create_path_handle<'a, A: Asset>(
        &self,
        path: impl Into<AssetPath<'a>>,
        meta_transform: Option<MetaTransform>,
    ) -> Handle<A> {
        let mut infos = self.data.infos.write();
        infos
            .get_or_create_path_handle::<A>(
                path.into().into_owned(),
                HandleLoadingMode::NotLoading,
                meta_transform,
            )
            .0
    }

    pub(crate) async fn get_meta_loader_and_reader<'a>(
        &'a self,
        asset_path: &'a AssetPath<'_>,
    ) -> Result<
        (
            Box<dyn AssetMetaDyn>,
            Arc<dyn ErasedAssetLoader>,
            Box<Reader<'a>>,
        ),
        AssetLoadError,
    > {
        // NOTE: We grab the asset byte reader first to ensure this is transactional for AssetReaders like ProcessorGatedReader
        // The asset byte reader will "lock" the processed asset, preventing writes for the duration of the lock.
        // Then the meta reader, if meta exists, will correspond to the meta for the current "version" of the asset.
        // See ProcessedAssetInfo::file_transaction_lock for more context
        let reader = self.data.reader.read(asset_path.path()).await?;
        match self.data.reader.read_meta_bytes(asset_path.path()).await {
            Ok(meta_bytes) => {
                // TODO: this isn't fully minimal yet. we only need the loader
                let minimal: AssetMetaMinimal = ron::de::from_bytes(&meta_bytes).map_err(|e| {
                    AssetLoadError::DeserializeMeta(DeserializeMetaError::DeserializeMinimal(e))
                })?;
                let loader_name = match minimal.asset {
                    AssetActionMinimal::Load { loader } => loader,
                    AssetActionMinimal::Process { .. } => {
                        return Err(AssetLoadError::CannotLoadProcessedAsset {
                            path: asset_path.clone().into_owned(),
                        })
                    }
                    AssetActionMinimal::Ignore => {
                        return Err(AssetLoadError::CannotLoadIgnoredAsset {
                            path: asset_path.clone().into_owned(),
                        })
                    }
                };
                let loader = self.get_asset_loader_with_type_name(&loader_name).await?;
                let meta = loader.deserialize_meta(&meta_bytes).map_err(|e| {
                    AssetLoadError::AssetLoaderError {
                        path: asset_path.clone().into_owned(),
                        loader: loader.type_name(),
                        error: AssetLoaderError::DeserializeMeta(e),
                    }
                })?;

                Ok((meta, loader, reader))
            }
            Err(AssetReaderError::NotFound(_)) => {
                let loader = self.get_path_asset_loader(asset_path).await?;
                let meta = loader.default_meta();
                Ok((meta, loader, reader))
            }
            Err(err) => Err(err.into()),
        }
    }

    pub(crate) async fn load_with_meta_loader_and_reader(
        &self,
        asset_path: &AssetPath<'_>,
        meta: Box<dyn AssetMetaDyn>,
        loader: &dyn ErasedAssetLoader,
        reader: &mut Reader<'_>,
        load_dependencies: bool,
        populate_hashes: bool,
    ) -> Result<ErasedLoadedAsset, AssetLoadError> {
        // TODO: experiment with this
        let asset_path = asset_path.clone().into_owned();
        let load_context =
            LoadContext::new(self, asset_path.clone(), load_dependencies, populate_hashes);
        loader.load(reader, meta, load_context).await.map_err(|e| {
            AssetLoadError::AssetLoaderError {
                loader: loader.type_name(),
                path: asset_path,
                error: e,
            }
        })
    }
}

/// A system that manages internal [`AssetServer`] events, such as finalizing asset loads.
pub fn handle_internal_asset_events(world: &mut World) {
    world.resource_scope(|world, server: Mut<AssetServer>| {
        let mut infos = server.data.infos.write();
        for event in server.data.asset_event_receiver.try_iter() {
            match event {
                InternalAssetEvent::Loaded { id, loaded_asset } => {
                    infos.process_asset_load(
                        id,
                        loaded_asset,
                        world,
                        &server.data.asset_event_sender,
                    );
                }
                InternalAssetEvent::LoadedWithDependencies { id } => {
                    let sender = infos
                        .dependency_loaded_event_sender
                        .get(&id.type_id())
                        .expect("Asset event sender should exist");
                    sender(world, id);
                }
                InternalAssetEvent::Failed { id } => infos.process_asset_fail(id),
            }
        }

        fn queue_ancestors(
            asset_path: &AssetPath,
            infos: &AssetInfos,
            paths_to_reload: &mut HashSet<AssetPath<'static>>,
        ) {
            if let Some(dependants) = infos.loader_dependants.get(asset_path) {
                for dependant in dependants {
                    paths_to_reload.insert(dependant.to_owned());
                    queue_ancestors(dependant, infos, paths_to_reload);
                }
            }
        }

        let mut paths_to_reload = HashSet::new();
        for event in server.data.source_event_receiver.try_iter() {
            match event {
                // TODO: if the asset was processed and the processed file was changed, the first modified event
                // should be skipped?
                AssetSourceEvent::ModifiedAsset(path) | AssetSourceEvent::ModifiedMeta(path) => {
                    let path = AssetPath::from_path(path);
                    queue_ancestors(&path, &infos, &mut paths_to_reload);
                    paths_to_reload.insert(path);
                }
                _ => {}
            }
        }

        for path in paths_to_reload {
            server.reload(path);
        }
    });
}

#[derive(Default)]
pub(crate) struct AssetLoaders {
    values: Vec<MaybeAssetLoader>,
    extension_to_index: HashMap<String, usize>,
    type_name_to_index: HashMap<&'static str, usize>,
    preregistered_loaders: HashMap<&'static str, usize>,
}

#[derive(Clone)]
enum MaybeAssetLoader {
    Ready(Arc<dyn ErasedAssetLoader>),
    Pending {
        sender: async_broadcast::Sender<Arc<dyn ErasedAssetLoader>>,
        receiver: async_broadcast::Receiver<Arc<dyn ErasedAssetLoader>>,
    },
}

/// Internal events for asset load results  
#[allow(clippy::large_enum_variant)]
pub(crate) enum InternalAssetEvent {
    Loaded {
        id: UntypedAssetId,
        loaded_asset: ErasedLoadedAsset,
    },
    LoadedWithDependencies {
        id: UntypedAssetId,
    },
    Failed {
        id: UntypedAssetId,
    },
}

/// The load state of an asset.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum LoadState {
    /// The asset has not started loading yet
    NotLoaded,
    /// The asset is in the process of loading.
    Loading,
    /// The asset has been loaded and has been added to the [`World`]
    Loaded,
    /// The asset failed to load.
    Failed,
}

/// The load state of an asset's dependencies.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DependencyLoadState {
    /// The asset has not started loading yet
    NotLoaded,
    /// Dependencies are still loading
    Loading,
    /// Dependencies have all loaded
    Loaded,
    /// One or more dependencies have failed to load
    Failed,
}

/// The recursive load state of an asset's dependencies.
#[derive(Component, Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RecursiveDependencyLoadState {
    /// The asset has not started loading yet
    NotLoaded,
    /// Dependencies in this asset's dependency tree are still loading
    Loading,
    /// Dependencies in this asset's dependency tree have all loaded
    Loaded,
    /// One or more dependencies have failed to load in this asset's dependency tree
    Failed,
}

/// An error that occurs during an [`Asset`] load.
#[derive(Error, Debug)]
pub enum AssetLoadError {
    #[error("Requested handle of type {requested:?} for asset '{path}' does not match actual asset type '{actual_asset_name}', which used loader '{loader_name}'")]
    RequestedHandleTypeMismatch {
        path: AssetPath<'static>,
        requested: TypeId,
        actual_asset_name: &'static str,
        loader_name: &'static str,
    },
    #[error(transparent)]
    MissingAssetLoaderForExtension(#[from] MissingAssetLoaderForExtensionError),
    #[error(transparent)]
    MissingAssetLoaderForTypeName(#[from] MissingAssetLoaderForTypeNameError),
    #[error(transparent)]
    AssetReaderError(#[from] AssetReaderError),
    #[error("Encountered an error while reading asset metadata bytes")]
    AssetMetaReadError,
    #[error(transparent)]
    DeserializeMeta(DeserializeMetaError),
    #[error("Asset '{path}' is configured to be processed. It cannot be loaded directly.")]
    CannotLoadProcessedAsset { path: AssetPath<'static> },
    #[error("Asset '{path}' is configured to be ignored. It cannot be loaded.")]
    CannotLoadIgnoredAsset { path: AssetPath<'static> },
    #[error("Asset '{path}' encountered an error in {loader}: {error}")]
    AssetLoaderError {
        path: AssetPath<'static>,
        loader: &'static str,
        error: AssetLoaderError,
    },
}

/// An error that occurs when an [`AssetLoader`] is not registered for a given extension.
#[derive(Error, Debug)]
#[error("no `AssetLoader` found{}", format_missing_asset_ext(.extensions))]
pub struct MissingAssetLoaderForExtensionError {
    extensions: Vec<String>,
}

/// An error that occurs when an [`AssetLoader`] is not registered for a given [`std::any::type_name`].
#[derive(Error, Debug)]
#[error("no `AssetLoader` found with the name '{type_name}'")]
pub struct MissingAssetLoaderForTypeNameError {
    type_name: String,
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

impl std::fmt::Debug for AssetServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetServer")
            .field("info", &self.data.infos.read())
            .finish()
    }
}

pub(crate) static CANNOT_WATCH_ERROR_MESSAGE: &str =
    "Cannot watch for changes because the current `AssetReader` does not support it. If you are using \
    the FileAssetReader (the default on desktop platforms), enabling the filesystem_watcher feature will \
    add this functionality.";
