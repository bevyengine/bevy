mod info;

use crate::{
    folder::LoadedFolder,
    io::{
        AssetReader, AssetReaderError, AssetSource, AssetSourceEvent, AssetSourceId, AssetSources,
        MissingAssetSourceError, MissingProcessedAssetReaderError, Reader,
    },
    loader::{AssetLoader, ErasedAssetLoader, LoadContext, LoadedAsset},
    meta::{
        loader_settings_meta_transform, AssetActionMinimal, AssetMetaDyn, AssetMetaMinimal,
        MetaTransform, Settings,
    },
    path::AssetPath,
    Asset, AssetEvent, AssetHandleProvider, AssetId, Assets, DeserializeMetaError,
    ErasedLoadedAsset, Handle, LoadedUntypedAsset, UntypedAssetId, UntypedHandle,
};
use bevy_ecs::prelude::*;
use bevy_log::{error, info, warn};
use bevy_tasks::IoTaskPool;
use bevy_utils::{CowArc, HashMap, HashSet};
use crossbeam_channel::{Receiver, Sender};
use futures_lite::StreamExt;
use info::*;
use parking_lot::RwLock;
use std::path::PathBuf;
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
    sources: AssetSources,
    mode: AssetServerMode,
}

/// The "asset mode" the server is currently in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetServerMode {
    /// This server loads unprocessed assets.
    Unprocessed,
    /// This server loads processed assets.
    Processed,
}

impl AssetServer {
    /// Create a new instance of [`AssetServer`]. If `watch_for_changes` is true, the [`AssetReader`] storage will watch for changes to
    /// asset sources and hot-reload them.
    pub fn new(sources: AssetSources, mode: AssetServerMode, watching_for_changes: bool) -> Self {
        Self::new_with_loaders(sources, Default::default(), mode, watching_for_changes)
    }

    pub(crate) fn new_with_loaders(
        sources: AssetSources,
        loaders: Arc<RwLock<AssetLoaders>>,
        mode: AssetServerMode,
        watching_for_changes: bool,
    ) -> Self {
        let (asset_event_sender, asset_event_receiver) = crossbeam_channel::unbounded();
        let mut infos = AssetInfos::default();
        infos.watching_for_changes = watching_for_changes;
        Self {
            data: Arc::new(AssetServerData {
                sources,
                mode,
                asset_event_sender,
                asset_event_receiver,
                loaders,
                infos: RwLock::new(infos),
            }),
        }
    }

    /// Retrieves the [`AssetReader`] for the given `source`.
    pub fn get_source<'a>(
        &'a self,
        source: impl Into<AssetSourceId<'a>>,
    ) -> Result<&'a AssetSource, MissingAssetSourceError> {
        self.data.sources.get(source.into())
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
        let path = path.into().into_owned();
        let (handle, should_load) = self.data.infos.write().get_or_create_path_handle::<A>(
            path.clone(),
            HandleLoadingMode::Request,
            meta_transform,
        );

        if should_load {
            let owned_handle = Some(handle.clone().untyped());
            let server = self.clone();
            IoTaskPool::get()
                .spawn(async move {
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

    /// Load an asset without knowing it's type. The method returns a handle to a [`LoadedUntypedAsset`].
    ///
    /// Once the [`LoadedUntypedAsset`] is loaded, an untyped handle for the requested path can be
    /// retrieved from it.
    ///
    /// ```
    /// use bevy_asset::{Assets, Handle, LoadedUntypedAsset};
    /// use bevy_ecs::system::{Res, Resource};
    ///
    /// #[derive(Resource)]
    /// struct LoadingUntypedHandle(Handle<LoadedUntypedAsset>);
    ///
    /// fn resolve_loaded_untyped_handle(loading_handle: Res<LoadingUntypedHandle>, loaded_untyped_assets: Res<Assets<LoadedUntypedAsset>>) {
    ///     if let Some(loaded_untyped_asset) = loaded_untyped_assets.get(&loading_handle.0) {
    ///         let handle = loaded_untyped_asset.handle.clone();
    ///         // continue working with `handle` which points to the asset at the originally requested path
    ///     }
    /// }
    /// ```
    ///
    /// This indirection enables a non blocking load of an untyped asset, since I/O is
    /// required to figure out the asset type before a handle can be created.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the assets"]
    pub fn load_untyped<'a>(&self, path: impl Into<AssetPath<'a>>) -> Handle<LoadedUntypedAsset> {
        let path = path.into().into_owned();
        let untyped_source = AssetSourceId::Name(match path.source() {
            AssetSourceId::Default => CowArc::Borrowed(UNTYPED_SOURCE_SUFFIX),
            AssetSourceId::Name(source) => {
                CowArc::Owned(format!("{source}--{UNTYPED_SOURCE_SUFFIX}").into())
            }
        });
        let (handle, should_load) = self
            .data
            .infos
            .write()
            .get_or_create_path_handle::<LoadedUntypedAsset>(
                path.clone().with_source(untyped_source),
                HandleLoadingMode::Request,
                None,
            );
        if !should_load {
            return handle;
        }
        let id = handle.id().untyped();

        let server = self.clone();
        IoTaskPool::get()
            .spawn(async move {
                match server.load_untyped_async(path).await {
                    Ok(handle) => server.send_asset_event(InternalAssetEvent::Loaded {
                        id,
                        loaded_asset: LoadedAsset::new_with_dependencies(
                            LoadedUntypedAsset { handle },
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

    /// Performs an async asset load.
    ///
    /// `input_handle` must only be [`Some`] if `should_load` was true when retrieving `input_handle`. This is an optimization to
    /// avoid looking up `should_load` twice, but it means you _must_ be sure a load is necessary when calling this function with [`Some`].
    async fn load_internal<'a>(
        &self,
        input_handle: Option<UntypedHandle>,
        path: AssetPath<'a>,
        force: bool,
        meta_transform: Option<MetaTransform>,
    ) -> Result<UntypedHandle, AssetLoadError> {
        let path = path.into_owned();
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

        let (handle, should_load) = match input_handle {
            Some(handle) => {
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

        if path.label().is_none() && handle.type_id() != loader.asset_type_id() {
            return Err(AssetLoadError::RequestedHandleTypeMismatch {
                path: path.into_owned(),
                requested: handle.type_id(),
                actual_asset_name: loader.asset_type_name(),
                loader_name: loader.type_name(),
            });
        }

        if !should_load && !force {
            return Ok(handle);
        }

        let (base_handle, base_path) = if path.label().is_some() {
            let mut infos = self.data.infos.write();
            let base_path = path.without_label().into_owned();
            let (base_handle, _) = infos.get_or_create_path_handle_untyped(
                base_path.clone(),
                loader.asset_type_id(),
                loader.asset_type_name(),
                HandleLoadingMode::Force,
                None,
            );
            (base_handle, base_path)
        } else {
            (handle.clone(), path.clone())
        };

        if let Some(meta_transform) = base_handle.meta_transform() {
            (*meta_transform)(&mut *meta);
        }

        match self
            .load_with_meta_loader_and_reader(&base_path, meta, &*loader, &mut *reader, true, false)
            .await
        {
            Ok(mut loaded_asset) => {
                if let Some(label) = path.label_cow() {
                    if !loaded_asset.labeled_assets.contains_key(&label) {
                        return Err(AssetLoadError::MissingLabel {
                            base_path,
                            label: label.to_string(),
                        });
                    }
                }
                for (_, labeled_asset) in loaded_asset.labeled_assets.drain() {
                    self.send_asset_event(InternalAssetEvent::Loaded {
                        id: labeled_asset.handle.id(),
                        loaded_asset: labeled_asset.asset,
                    });
                }
                self.send_asset_event(InternalAssetEvent::Loaded {
                    id: base_handle.id(),
                    loaded_asset,
                });
                Ok(handle)
            }
            Err(err) => {
                self.send_asset_event(InternalAssetEvent::Failed {
                    id: base_handle.id(),
                });
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
                if server.data.infos.read().should_reload(&path) {
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
    ///
    /// Loading the same folder multiple times will return the same handle. If the `file_watcher`
    /// feature is enabled, [`LoadedFolder`] handles will reload when a file in the folder is
    /// removed, added or moved. This includes files in subdirectories and moving, adding,
    /// or removing complete subdirectories.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the assets"]
    pub fn load_folder<'a>(&self, path: impl Into<AssetPath<'a>>) -> Handle<LoadedFolder> {
        let path = path.into().into_owned();
        let (handle, should_load) = self
            .data
            .infos
            .write()
            .get_or_create_path_handle::<LoadedFolder>(
                path.clone(),
                HandleLoadingMode::Request,
                None,
            );
        if !should_load {
            return handle;
        }
        let id = handle.id().untyped();
        self.load_folder_internal(id, path);

        handle
    }

    pub(crate) fn load_folder_internal(&self, id: UntypedAssetId, path: AssetPath) {
        fn load_folder<'a>(
            source: AssetSourceId<'static>,
            path: &'a Path,
            reader: &'a dyn AssetReader,
            server: &'a AssetServer,
            handles: &'a mut Vec<UntypedHandle>,
        ) -> bevy_utils::BoxedFuture<'a, Result<(), AssetLoadError>> {
            Box::pin(async move {
                let is_dir = reader.is_directory(path).await?;
                if is_dir {
                    let mut path_stream = reader.read_directory(path.as_ref()).await?;
                    while let Some(child_path) = path_stream.next().await {
                        if reader.is_directory(&child_path).await? {
                            load_folder(source.clone(), &child_path, reader, server, handles)
                                .await?;
                        } else {
                            let path = child_path.to_str().expect("Path should be a valid string.");
                            let asset_path = AssetPath::parse(path).with_source(source.clone());
                            match server.load_untyped_async(asset_path).await {
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

        let path = path.into_owned();
        let server = self.clone();
        IoTaskPool::get()
            .spawn(async move {
                let Ok(source) = server.get_source(path.source()) else {
                    error!(
                        "Failed to load {path}. AssetSource {:?} does not exist",
                        path.source()
                    );
                    return;
                };

                let asset_reader = match server.data.mode {
                    AssetServerMode::Unprocessed { .. } => source.reader(),
                    AssetServerMode::Processed { .. } => match source.processed_reader() {
                        Ok(reader) => reader,
                        Err(_) => {
                            error!(
                                "Failed to load {path}. AssetSource {:?} does not have a processed AssetReader",
                                path.source()
                            );
                            return;
                        }
                    },
                };

                let mut handles = Vec::new();
                match load_folder(source.id(), path.path(), asset_reader, &server, &mut handles).await {
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

    /// Returns the [`AssetServerMode`] this server is currently in.
    pub fn mode(&self) -> AssetServerMode {
        self.data.mode
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
        let source = self.get_source(asset_path.source())?;
        // NOTE: We grab the asset byte reader first to ensure this is transactional for AssetReaders like ProcessorGatedReader
        // The asset byte reader will "lock" the processed asset, preventing writes for the duration of the lock.
        // Then the meta reader, if meta exists, will correspond to the meta for the current "version" of the asset.
        // See ProcessedAssetInfo::file_transaction_lock for more context
        let asset_reader = match self.data.mode {
            AssetServerMode::Unprocessed { .. } => source.reader(),
            AssetServerMode::Processed { .. } => source.processed_reader()?,
        };
        let reader = asset_reader.read(asset_path.path()).await?;
        match asset_reader.read_meta_bytes(asset_path.path()).await {
            Ok(meta_bytes) => {
                // TODO: this isn't fully minimal yet. we only need the loader
                let minimal: AssetMetaMinimal = ron::de::from_bytes(&meta_bytes).map_err(|e| {
                    AssetLoadError::DeserializeMeta {
                        path: asset_path.clone_owned(),
                        error: Box::new(DeserializeMetaError::DeserializeMinimal(e)),
                    }
                })?;
                let loader_name = match minimal.asset {
                    AssetActionMinimal::Load { loader } => loader,
                    AssetActionMinimal::Process { .. } => {
                        return Err(AssetLoadError::CannotLoadProcessedAsset {
                            path: asset_path.clone_owned(),
                        })
                    }
                    AssetActionMinimal::Ignore => {
                        return Err(AssetLoadError::CannotLoadIgnoredAsset {
                            path: asset_path.clone_owned(),
                        })
                    }
                };
                let loader = self.get_asset_loader_with_type_name(&loader_name).await?;
                let meta = loader.deserialize_meta(&meta_bytes).map_err(|e| {
                    AssetLoadError::DeserializeMeta {
                        path: asset_path.clone_owned(),
                        error: Box::new(e),
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
        let asset_path = asset_path.clone_owned();
        let load_context =
            LoadContext::new(self, asset_path.clone(), load_dependencies, populate_hashes);
        loader.load(reader, meta, load_context).await.map_err(|e| {
            AssetLoadError::AssetLoaderError {
                path: asset_path.clone_owned(),
                loader_name: loader.type_name(),
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

        let reload_parent_folders = |path: PathBuf, source: &AssetSourceId<'static>| {
            let mut current_folder = path;
            while let Some(parent) = current_folder.parent() {
                current_folder = parent.to_path_buf();
                let parent_asset_path =
                    AssetPath::from(current_folder.clone()).with_source(source.clone());
                if let Some(folder_handle) = infos.get_path_handle(parent_asset_path.clone()) {
                    info!("Reloading folder {parent_asset_path} because the content has changed");
                    server.load_folder_internal(folder_handle.id(), parent_asset_path);
                }
            }
        };

        let mut paths_to_reload = HashSet::new();
        let mut handle_event = |source: AssetSourceId<'static>, event: AssetSourceEvent| {
            match event {
                // TODO: if the asset was processed and the processed file was changed, the first modified event
                // should be skipped?
                AssetSourceEvent::ModifiedAsset(path) | AssetSourceEvent::ModifiedMeta(path) => {
                    let path = AssetPath::from(path).with_source(source);
                    queue_ancestors(&path, &infos, &mut paths_to_reload);
                    paths_to_reload.insert(path);
                }
                AssetSourceEvent::RenamedFolder { old, new } => {
                    reload_parent_folders(old, &source);
                    reload_parent_folders(new, &source);
                }
                AssetSourceEvent::AddedAsset(path)
                | AssetSourceEvent::RemovedAsset(path)
                | AssetSourceEvent::RemovedFolder(path)
                | AssetSourceEvent::AddedFolder(path) => {
                    reload_parent_folders(path, &source);
                }
                _ => {}
            }
        };

        for source in server.data.sources.iter() {
            match server.data.mode {
                AssetServerMode::Unprocessed { .. } => {
                    if let Some(receiver) = source.event_receiver() {
                        for event in receiver.try_iter() {
                            handle_event(source.id(), event);
                        }
                    }
                }
                AssetServerMode::Processed { .. } => {
                    if let Some(receiver) = source.processed_event_receiver() {
                        for event in receiver.try_iter() {
                            handle_event(source.id(), event);
                        }
                    }
                }
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
    #[error(transparent)]
    MissingAssetSourceError(#[from] MissingAssetSourceError),
    #[error(transparent)]
    MissingProcessedAssetReaderError(#[from] MissingProcessedAssetReaderError),
    #[error("Encountered an error while reading asset metadata bytes")]
    AssetMetaReadError,
    #[error("Failed to deserialize meta for asset {path}: {error}")]
    DeserializeMeta {
        path: AssetPath<'static>,
        error: Box<DeserializeMetaError>,
    },
    #[error("Asset '{path}' is configured to be processed. It cannot be loaded directly.")]
    CannotLoadProcessedAsset { path: AssetPath<'static> },
    #[error("Asset '{path}' is configured to be ignored. It cannot be loaded.")]
    CannotLoadIgnoredAsset { path: AssetPath<'static> },
    #[error("Failed to load asset '{path}' with asset loader '{loader_name}': {error}")]
    AssetLoaderError {
        path: AssetPath<'static>,
        loader_name: &'static str,
        error: Box<dyn std::error::Error + Send + Sync + 'static>,
    },
    #[error("The file at '{base_path}' does not contain the labeled asset '{label}'.")]
    MissingLabel {
        base_path: AssetPath<'static>,
        label: String,
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
        " for file with no extension".to_string()
    }
}

impl std::fmt::Debug for AssetServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetServer")
            .field("info", &self.data.infos.read())
            .finish()
    }
}

/// This is appended to asset sources when loading a [`LoadedUntypedAsset`]. This provides a unique
/// source for a given [`AssetPath`].
const UNTYPED_SOURCE_SUFFIX: &str = "--untyped";
