mod info;
mod loaders;

use crate::{
    folder::LoadedFolder,
    io::{
        AssetReaderError, AssetSource, AssetSourceEvent, AssetSourceId, AssetSources,
        AssetWriterError, ErasedAssetReader, MissingAssetSourceError, MissingAssetWriterError,
        MissingProcessedAssetReaderError, Reader,
    },
    loader::{AssetLoader, ErasedAssetLoader, LoadContext, LoadedAsset},
    meta::{
        loader_settings_meta_transform, AssetActionMinimal, AssetMetaDyn, AssetMetaMinimal,
        MetaTransform, Settings,
    },
    path::AssetPath,
    Asset, AssetEvent, AssetHandleProvider, AssetId, AssetLoadFailedEvent, AssetMetaCheck, Assets,
    DeserializeMetaError, ErasedLoadedAsset, Handle, LoadedUntypedAsset, UnapprovedPathMode,
    UntypedAssetId, UntypedAssetLoadFailedEvent, UntypedHandle,
};
use alloc::{borrow::ToOwned, boxed::Box, vec, vec::Vec};
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
};
use atomicow::CowArc;
use bevy_ecs::prelude::*;
use bevy_platform::collections::HashSet;
use bevy_tasks::IoTaskPool;
use core::{any::TypeId, future::Future, panic::AssertUnwindSafe, task::Poll};
use crossbeam_channel::{Receiver, Sender};
use either::Either;
use futures_lite::{FutureExt, StreamExt};
use info::*;
use loaders::*;
use parking_lot::{RwLock, RwLockWriteGuard};
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{error, info};

/// Loads and tracks the state of [`Asset`] values from a configured [`AssetReader`](crate::io::AssetReader).
/// This can be used to kick off new asset loads and retrieve their current load states.
///
/// The general process to load an asset is:
/// 1. Initialize a new [`Asset`] type with the [`AssetServer`] via [`AssetApp::init_asset`], which
///    will internally call [`AssetServer::register_asset`] and set up related ECS [`Assets`]
///    storage and systems.
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
    meta_check: AssetMetaCheck,
    unapproved_path_mode: UnapprovedPathMode,
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
    /// Create a new instance of [`AssetServer`]. If `watch_for_changes` is true, the [`AssetReader`](crate::io::AssetReader) storage will watch for changes to
    /// asset sources and hot-reload them.
    pub fn new(
        sources: AssetSources,
        mode: AssetServerMode,
        watching_for_changes: bool,
        unapproved_path_mode: UnapprovedPathMode,
    ) -> Self {
        Self::new_with_loaders(
            sources,
            Default::default(),
            mode,
            AssetMetaCheck::Always,
            watching_for_changes,
            unapproved_path_mode,
        )
    }

    /// Create a new instance of [`AssetServer`]. If `watch_for_changes` is true, the [`AssetReader`](crate::io::AssetReader) storage will watch for changes to
    /// asset sources and hot-reload them.
    pub fn new_with_meta_check(
        sources: AssetSources,
        mode: AssetServerMode,
        meta_check: AssetMetaCheck,
        watching_for_changes: bool,
        unapproved_path_mode: UnapprovedPathMode,
    ) -> Self {
        Self::new_with_loaders(
            sources,
            Default::default(),
            mode,
            meta_check,
            watching_for_changes,
            unapproved_path_mode,
        )
    }

    pub(crate) fn new_with_loaders(
        sources: AssetSources,
        loaders: Arc<RwLock<AssetLoaders>>,
        mode: AssetServerMode,
        meta_check: AssetMetaCheck,
        watching_for_changes: bool,
        unapproved_path_mode: UnapprovedPathMode,
    ) -> Self {
        let (asset_event_sender, asset_event_receiver) = crossbeam_channel::unbounded();
        let mut infos = AssetInfos::default();
        infos.watching_for_changes = watching_for_changes;
        Self {
            data: Arc::new(AssetServerData {
                sources,
                mode,
                meta_check,
                asset_event_sender,
                asset_event_receiver,
                loaders,
                infos: RwLock::new(infos),
                unapproved_path_mode,
            }),
        }
    }

    /// Retrieves the [`AssetSource`] for the given `source`.
    pub fn get_source<'a>(
        &self,
        source: impl Into<AssetSourceId<'a>>,
    ) -> Result<&AssetSource, MissingAssetSourceError> {
        self.data.sources.get(source.into())
    }

    /// Returns true if the [`AssetServer`] watches for changes.
    pub fn watching_for_changes(&self) -> bool {
        self.data.infos.read().watching_for_changes
    }

    /// Registers a new [`AssetLoader`]. [`AssetLoader`]s must be registered before they can be used.
    pub fn register_loader<L: AssetLoader>(&self, loader: L) {
        self.data.loaders.write().push(loader);
    }

    /// Registers a new [`Asset`] type. [`Asset`] types must be registered before assets of that type can be loaded.
    pub fn register_asset<A: Asset>(&self, assets: &Assets<A>) {
        self.register_handle_provider(assets.get_handle_provider());
        fn sender<A: Asset>(world: &mut World, id: UntypedAssetId) {
            world
                .resource_mut::<Events<AssetEvent<A>>>()
                .write(AssetEvent::LoadedWithDependencies { id: id.typed() });
        }
        fn failed_sender<A: Asset>(
            world: &mut World,
            id: UntypedAssetId,
            path: AssetPath<'static>,
            error: AssetLoadError,
        ) {
            world
                .resource_mut::<Events<AssetLoadFailedEvent<A>>>()
                .write(AssetLoadFailedEvent {
                    id: id.typed(),
                    path,
                    error,
                });
        }

        let mut infos = self.data.infos.write();

        infos
            .dependency_loaded_event_sender
            .insert(TypeId::of::<A>(), sender::<A>);

        infos
            .dependency_failed_event_sender
            .insert(TypeId::of::<A>(), failed_sender::<A>);
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
        let error = || MissingAssetLoaderForExtensionError {
            extensions: vec![extension.to_string()],
        };

        let loader = { self.data.loaders.read().get_by_extension(extension) };

        loader.ok_or_else(error)?.get().await.map_err(|_| error())
    }

    /// Returns the registered [`AssetLoader`] associated with the given [`core::any::type_name`], if it exists.
    pub async fn get_asset_loader_with_type_name(
        &self,
        type_name: &str,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForTypeNameError> {
        let error = || MissingAssetLoaderForTypeNameError {
            type_name: type_name.to_string(),
        };

        let loader = { self.data.loaders.read().get_by_name(type_name) };

        loader.ok_or_else(error)?.get().await.map_err(|_| error())
    }

    /// Retrieves the default [`AssetLoader`] for the given path, if one can be found.
    pub async fn get_path_asset_loader<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForExtensionError> {
        let path = path.into();

        let error = || {
            let Some(full_extension) = path.get_full_extension() else {
                return MissingAssetLoaderForExtensionError {
                    extensions: Vec::new(),
                };
            };

            let mut extensions = vec![full_extension.clone()];
            extensions.extend(
                AssetPath::iter_secondary_extensions(&full_extension).map(ToString::to_string),
            );

            MissingAssetLoaderForExtensionError { extensions }
        };

        let loader = { self.data.loaders.read().get_by_path(&path) };

        loader.ok_or_else(error)?.get().await.map_err(|_| error())
    }

    /// Retrieves the default [`AssetLoader`] for the given [`Asset`] [`TypeId`], if one can be found.
    pub async fn get_asset_loader_with_asset_type_id(
        &self,
        type_id: TypeId,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForTypeIdError> {
        let error = || MissingAssetLoaderForTypeIdError { type_id };

        let loader = { self.data.loaders.read().get_by_type(type_id) };

        loader.ok_or_else(error)?.get().await.map_err(|_| error())
    }

    /// Retrieves the default [`AssetLoader`] for the given [`Asset`] type, if one can be found.
    pub async fn get_asset_loader_with_asset_type<A: Asset>(
        &self,
    ) -> Result<Arc<dyn ErasedAssetLoader>, MissingAssetLoaderForTypeIdError> {
        self.get_asset_loader_with_asset_type_id(TypeId::of::<A>())
            .await
    }

    /// Begins loading an [`Asset`] of type `A` stored at `path`. This will not block on the asset load. Instead,
    /// it returns a "strong" [`Handle`]. When the [`Asset`] is loaded (and enters [`LoadState::Loaded`]), it will be added to the
    /// associated [`Assets`] resource.
    ///
    /// Note that if the asset at this path is already loaded, this function will return the existing handle,
    /// and will not waste work spawning a new load task.
    ///
    /// In case the file path contains a hashtag (`#`), the `path` must be specified using [`Path`]
    /// or [`AssetPath`] because otherwise the hashtag would be interpreted as separator between
    /// the file path and the label. For example:
    ///
    /// ```no_run
    /// # use bevy_asset::{AssetServer, Handle, LoadedUntypedAsset};
    /// # use bevy_ecs::prelude::Res;
    /// # use std::path::Path;
    /// // `#path` is a label.
    /// # fn setup(asset_server: Res<AssetServer>) {
    /// # let handle: Handle<LoadedUntypedAsset> =
    /// asset_server.load("some/file#path");
    ///
    /// // `#path` is part of the file name.
    /// # let handle: Handle<LoadedUntypedAsset> =
    /// asset_server.load(Path::new("some/file#path"));
    /// # }
    /// ```
    ///
    /// Furthermore, if you need to load a file with a hashtag in its name _and_ a label, you can
    /// manually construct an [`AssetPath`].
    ///
    /// ```no_run
    /// # use bevy_asset::{AssetPath, AssetServer, Handle, LoadedUntypedAsset};
    /// # use bevy_ecs::prelude::Res;
    /// # use std::path::Path;
    /// # fn setup(asset_server: Res<AssetServer>) {
    /// # let handle: Handle<LoadedUntypedAsset> =
    /// asset_server.load(AssetPath::from_path(Path::new("some/file#path")).with_label("subasset"));
    /// # }
    /// ```
    ///
    /// You can check the asset's load state by reading [`AssetEvent`] events, calling [`AssetServer::load_state`], or checking
    /// the [`Assets`] storage to see if the [`Asset`] exists yet.
    ///
    /// The asset load will fail and an error will be printed to the logs if the asset stored at `path` is not of type `A`.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A> {
        self.load_with_meta_transform(path, None, (), false)
    }

    /// Same as [`load`](AssetServer::load), but you can load assets from unaproved paths
    /// if [`AssetPlugin::unapproved_path_mode`](super::AssetPlugin::unapproved_path_mode)
    /// is [`Deny`](UnapprovedPathMode::Deny).
    ///
    /// See [`UnapprovedPathMode`] and [`AssetPath::is_unapproved`]
    pub fn load_override<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Handle<A> {
        self.load_with_meta_transform(path, None, (), true)
    }

    /// Begins loading an [`Asset`] of type `A` stored at `path` while holding a guard item.
    /// The guard item is dropped when either the asset is loaded or loading has failed.
    ///
    /// This function returns a "strong" [`Handle`]. When the [`Asset`] is loaded (and enters [`LoadState::Loaded`]), it will be added to the
    /// associated [`Assets`] resource.
    ///
    /// The guard item should notify the caller in its [`Drop`] implementation. See example `multi_asset_sync`.
    /// Synchronously this can be a [`Arc<AtomicU32>`] that decrements its counter, asynchronously this can be a `Barrier`.
    /// This function only guarantees the asset referenced by the [`Handle`] is loaded. If your asset is separated into
    /// multiple files, sub-assets referenced by the main asset might still be loading, depend on the implementation of the [`AssetLoader`].
    ///
    /// Additionally, you can check the asset's load state by reading [`AssetEvent`] events, calling [`AssetServer::load_state`], or checking
    /// the [`Assets`] storage to see if the [`Asset`] exists yet.
    ///
    /// The asset load will fail and an error will be printed to the logs if the asset stored at `path` is not of type `A`.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load_acquire<'a, A: Asset, G: Send + Sync + 'static>(
        &self,
        path: impl Into<AssetPath<'a>>,
        guard: G,
    ) -> Handle<A> {
        self.load_with_meta_transform(path, None, guard, false)
    }

    /// Same as [`load`](AssetServer::load_acquire), but you can load assets from unaproved paths
    /// if [`AssetPlugin::unapproved_path_mode`](super::AssetPlugin::unapproved_path_mode)
    /// is [`Deny`](UnapprovedPathMode::Deny).
    ///
    /// See [`UnapprovedPathMode`] and [`AssetPath::is_unapproved`]
    pub fn load_acquire_override<'a, A: Asset, G: Send + Sync + 'static>(
        &self,
        path: impl Into<AssetPath<'a>>,
        guard: G,
    ) -> Handle<A> {
        self.load_with_meta_transform(path, None, guard, true)
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
        self.load_with_meta_transform(
            path,
            Some(loader_settings_meta_transform(settings)),
            (),
            false,
        )
    }

    /// Same as [`load`](AssetServer::load_with_settings), but you can load assets from unaproved paths
    /// if [`AssetPlugin::unapproved_path_mode`](super::AssetPlugin::unapproved_path_mode)
    /// is [`Deny`](UnapprovedPathMode::Deny).
    ///
    /// See [`UnapprovedPathMode`] and [`AssetPath::is_unapproved`]
    pub fn load_with_settings_override<'a, A: Asset, S: Settings>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
    ) -> Handle<A> {
        self.load_with_meta_transform(
            path,
            Some(loader_settings_meta_transform(settings)),
            (),
            true,
        )
    }

    /// Begins loading an [`Asset`] of type `A` stored at `path` while holding a guard item.
    /// The guard item is dropped when either the asset is loaded or loading has failed.
    ///
    /// This function only guarantees the asset referenced by the [`Handle`] is loaded. If your asset is separated into
    /// multiple files, sub-assets referenced by the main asset might still be loading, depend on the implementation of the [`AssetLoader`].
    ///
    /// The given `settings` function will override the asset's
    /// [`AssetLoader`] settings. The type `S` _must_ match the configured [`AssetLoader::Settings`] or `settings` changes
    /// will be ignored and an error will be printed to the log.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load_acquire_with_settings<'a, A: Asset, S: Settings, G: Send + Sync + 'static>(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
        guard: G,
    ) -> Handle<A> {
        self.load_with_meta_transform(
            path,
            Some(loader_settings_meta_transform(settings)),
            guard,
            false,
        )
    }

    /// Same as [`load`](AssetServer::load_acquire_with_settings), but you can load assets from unaproved paths
    /// if [`AssetPlugin::unapproved_path_mode`](super::AssetPlugin::unapproved_path_mode)
    /// is [`Deny`](UnapprovedPathMode::Deny).
    ///
    /// See [`UnapprovedPathMode`] and [`AssetPath::is_unapproved`]
    pub fn load_acquire_with_settings_override<
        'a,
        A: Asset,
        S: Settings,
        G: Send + Sync + 'static,
    >(
        &self,
        path: impl Into<AssetPath<'a>>,
        settings: impl Fn(&mut S) + Send + Sync + 'static,
        guard: G,
    ) -> Handle<A> {
        self.load_with_meta_transform(
            path,
            Some(loader_settings_meta_transform(settings)),
            guard,
            true,
        )
    }

    pub(crate) fn load_with_meta_transform<'a, A: Asset, G: Send + Sync + 'static>(
        &self,
        path: impl Into<AssetPath<'a>>,
        meta_transform: Option<MetaTransform>,
        guard: G,
        override_unapproved: bool,
    ) -> Handle<A> {
        let path = path.into().into_owned();

        if path.is_unapproved() {
            match (&self.data.unapproved_path_mode, override_unapproved) {
                (UnapprovedPathMode::Allow, _) | (UnapprovedPathMode::Deny, true) => {}
                (UnapprovedPathMode::Deny, false) | (UnapprovedPathMode::Forbid, _) => {
                    error!("Asset path {path} is unapproved. See UnapprovedPathMode for details.");
                    return Handle::default();
                }
            }
        }

        let mut infos = self.data.infos.write();
        let (handle, should_load) = infos.get_or_create_path_handle::<A>(
            path.clone(),
            HandleLoadingMode::Request,
            meta_transform,
        );

        if should_load {
            self.spawn_load_task(handle.clone().untyped(), path, infos, guard);
        }

        handle
    }

    pub(crate) fn load_erased_with_meta_transform<'a, G: Send + Sync + 'static>(
        &self,
        path: impl Into<AssetPath<'a>>,
        type_id: TypeId,
        meta_transform: Option<MetaTransform>,
        guard: G,
    ) -> UntypedHandle {
        let path = path.into().into_owned();
        let mut infos = self.data.infos.write();
        let (handle, should_load) = infos.get_or_create_path_handle_erased(
            path.clone(),
            type_id,
            None,
            HandleLoadingMode::Request,
            meta_transform,
        );

        if should_load {
            self.spawn_load_task(handle.clone(), path, infos, guard);
        }

        handle
    }

    pub(crate) fn spawn_load_task<G: Send + Sync + 'static>(
        &self,
        handle: UntypedHandle,
        path: AssetPath<'static>,
        infos: RwLockWriteGuard<AssetInfos>,
        guard: G,
    ) {
        // drop the lock on `AssetInfos` before spawning a task that may block on it in single-threaded
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        drop(infos);

        let owned_handle = handle.clone();
        let server = self.clone();
        let task = IoTaskPool::get().spawn(async move {
            if let Err(err) = server
                .load_internal(Some(owned_handle), path, false, None)
                .await
            {
                error!("{}", err);
            }
            drop(guard);
        });

        #[cfg(not(any(target_arch = "wasm32", not(feature = "multi_threaded"))))]
        {
            let mut infos = infos;
            infos.pending_tasks.insert(handle.id(), task);
        }

        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        task.detach();
    }

    /// Asynchronously load an asset that you do not know the type of statically. If you _do_ know the type of the asset,
    /// you should use [`AssetServer::load`]. If you don't know the type of the asset, but you can't use an async method,
    /// consider using [`AssetServer::load_untyped`].
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub async fn load_untyped_async<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
    ) -> Result<UntypedHandle, AssetLoadError> {
        let path: AssetPath = path.into();
        self.load_internal(None, path, false, None)
            .await
            .map(|h| h.expect("handle must be returned, since we didn't pass in an input handle"))
    }

    pub(crate) fn load_unknown_type_with_meta_transform<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
        meta_transform: Option<MetaTransform>,
    ) -> Handle<LoadedUntypedAsset> {
        let path = path.into().into_owned();
        let untyped_source = AssetSourceId::Name(match path.source() {
            AssetSourceId::Default => CowArc::Static(UNTYPED_SOURCE_SUFFIX),
            AssetSourceId::Name(source) => {
                CowArc::Owned(format!("{source}--{UNTYPED_SOURCE_SUFFIX}").into())
            }
        });
        let mut infos = self.data.infos.write();
        let (handle, should_load) = infos.get_or_create_path_handle::<LoadedUntypedAsset>(
            path.clone().with_source(untyped_source),
            HandleLoadingMode::Request,
            meta_transform,
        );

        // drop the lock on `AssetInfos` before spawning a task that may block on it in single-threaded
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        drop(infos);

        if !should_load {
            return handle;
        }
        let id = handle.id().untyped();

        let server = self.clone();
        let task = IoTaskPool::get().spawn(async move {
            let path_clone = path.clone();
            match server.load_untyped_async(path).await {
                Ok(handle) => server.send_asset_event(InternalAssetEvent::Loaded {
                    id,
                    loaded_asset: LoadedAsset::new_with_dependencies(LoadedUntypedAsset { handle })
                        .into(),
                }),
                Err(err) => {
                    error!("{err}");
                    server.send_asset_event(InternalAssetEvent::Failed {
                        id,
                        path: path_clone,
                        error: err,
                    });
                }
            }
        });

        #[cfg(not(any(target_arch = "wasm32", not(feature = "multi_threaded"))))]
        infos.pending_tasks.insert(handle.id().untyped(), task);

        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        task.detach();

        handle
    }

    /// Load an asset without knowing its type. The method returns a handle to a [`LoadedUntypedAsset`].
    ///
    /// Once the [`LoadedUntypedAsset`] is loaded, an untyped handle for the requested path can be
    /// retrieved from it.
    ///
    /// ```
    /// use bevy_asset::{Assets, Handle, LoadedUntypedAsset};
    /// use bevy_ecs::system::Res;
    /// use bevy_ecs::resource::Resource;
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
        self.load_unknown_type_with_meta_transform(path, None)
    }

    /// Performs an async asset load.
    ///
    /// `input_handle` must only be [`Some`] if `should_load` was true when retrieving
    /// `input_handle`. This is an optimization to avoid looking up `should_load` twice, but it
    /// means you _must_ be sure a load is necessary when calling this function with [`Some`].
    ///
    /// Returns the handle of the asset if one was retrieved by this function. Otherwise, may return
    /// [`None`].
    async fn load_internal<'a>(
        &self,
        input_handle: Option<UntypedHandle>,
        path: AssetPath<'a>,
        force: bool,
        meta_transform: Option<MetaTransform>,
    ) -> Result<Option<UntypedHandle>, AssetLoadError> {
        let input_handle_type_id = input_handle.as_ref().map(UntypedHandle::type_id);

        let path = path.into_owned();
        let path_clone = path.clone();
        let (mut meta, loader, mut reader) = self
            .get_meta_loader_and_reader(&path_clone, input_handle_type_id)
            .await
            .inspect_err(|e| {
                // if there was an input handle, a "load" operation has already started, so we must produce a "failure" event, if
                // we cannot find the meta and loader
                if let Some(handle) = &input_handle {
                    self.send_asset_event(InternalAssetEvent::Failed {
                        id: handle.id(),
                        path: path.clone_owned(),
                        error: e.clone(),
                    });
                }
            })?;

        if let Some(meta_transform) = input_handle.as_ref().and_then(|h| h.meta_transform()) {
            (*meta_transform)(&mut *meta);
        }

        let asset_id; // The asset ID of the asset we are trying to load.
        let fetched_handle; // The handle if one was looked up/created.
        let should_load; // Whether we need to load the asset.
        if let Some(input_handle) = input_handle {
            asset_id = Some(input_handle.id());
            // In this case, we intentionally drop the input handle so we can cancel loading the
            // asset if the handle gets dropped (externally) before it finishes loading.
            fetched_handle = None;
            // The handle was passed in, so the "should_load" check was already done.
            should_load = true;
        } else {
            // TODO: multiple asset loads for the same path can happen at the same time (rather than
            // "early out-ing" in the "normal" case). This would be resolved by a universal asset
            // id, as we would not need to resolve the asset type to generate the ID. See this
            // issue: https://github.com/bevyengine/bevy/issues/10549

            let mut infos = self.data.infos.write();
            let result = infos.get_or_create_path_handle_internal(
                path.clone(),
                path.label().is_none().then(|| loader.asset_type_id()),
                HandleLoadingMode::Request,
                meta_transform,
            );
            match unwrap_with_context(result, Either::Left(loader.asset_type_name())) {
                // We couldn't figure out the correct handle without its type ID (which can only
                // happen if we are loading a subasset).
                None => {
                    // We don't know the expected type since the subasset may have a different type
                    // than the "root" asset (which is the type the loader will load).
                    asset_id = None;
                    fetched_handle = None;
                    // If we couldn't find an appropriate handle, then the asset certainly needs to
                    // be loaded.
                    should_load = true;
                }
                Some((handle, result_should_load)) => {
                    asset_id = Some(handle.id());
                    fetched_handle = Some(handle);
                    should_load = result_should_load;
                }
            }
        }
        // Verify that the expected type matches the loader's type.
        if let Some(asset_type_id) = asset_id.map(|id| id.type_id()) {
            // If we are loading a subasset, then the subasset's type almost certainly doesn't match
            // the loader's type - and that's ok.
            if path.label().is_none() && asset_type_id != loader.asset_type_id() {
                error!(
                    "Expected {:?}, got {:?}",
                    asset_type_id,
                    loader.asset_type_id()
                );
                return Err(AssetLoadError::RequestedHandleTypeMismatch {
                    path: path.into_owned(),
                    requested: asset_type_id,
                    actual_asset_name: loader.asset_type_name(),
                    loader_name: loader.type_name(),
                });
            }
        }
        // Bail out earlier if we don't need to load the asset.
        if !should_load && !force {
            return Ok(fetched_handle);
        }

        // We don't actually need to use _base_handle, but we do need to keep the handle alive.
        // Dropping it would cancel the load of the base asset, which would make the load of this
        // subasset never complete.
        let (base_asset_id, _base_handle, base_path) = if path.label().is_some() {
            let mut infos = self.data.infos.write();
            let base_path = path.without_label().into_owned();
            let base_handle = infos
                .get_or_create_path_handle_erased(
                    base_path.clone(),
                    loader.asset_type_id(),
                    Some(loader.asset_type_name()),
                    HandleLoadingMode::Force,
                    None,
                )
                .0;
            (base_handle.id(), Some(base_handle), base_path)
        } else {
            (asset_id.unwrap(), None, path.clone())
        };

        match self
            .load_with_meta_loader_and_reader(
                &base_path,
                meta.as_ref(),
                &*loader,
                &mut *reader,
                true,
                false,
            )
            .await
        {
            Ok(loaded_asset) => {
                let final_handle = if let Some(label) = path.label_cow() {
                    match loaded_asset.labeled_assets.get(&label) {
                        Some(labeled_asset) => Some(labeled_asset.handle.clone()),
                        None => {
                            let mut all_labels: Vec<String> = loaded_asset
                                .labeled_assets
                                .keys()
                                .map(|s| (**s).to_owned())
                                .collect();
                            all_labels.sort_unstable();
                            return Err(AssetLoadError::MissingLabel {
                                base_path,
                                label: label.to_string(),
                                all_labels,
                            });
                        }
                    }
                } else {
                    fetched_handle
                };

                self.send_loaded_asset(base_asset_id, loaded_asset);
                Ok(final_handle)
            }
            Err(err) => {
                self.send_asset_event(InternalAssetEvent::Failed {
                    id: base_asset_id,
                    error: err.clone(),
                    path: path.into_owned(),
                });
                Err(err)
            }
        }
    }

    /// Sends a load event for the given `loaded_asset` and does the same recursively for all
    /// labeled assets.
    fn send_loaded_asset(&self, id: UntypedAssetId, mut loaded_asset: ErasedLoadedAsset) {
        for (_, labeled_asset) in loaded_asset.labeled_assets.drain() {
            self.send_loaded_asset(labeled_asset.handle.id(), labeled_asset.asset);
        }

        self.send_asset_event(InternalAssetEvent::Loaded { id, loaded_asset });
    }

    /// Kicks off a reload of the asset stored at the given path. This will only reload the asset if it currently loaded.
    pub fn reload<'a>(&self, path: impl Into<AssetPath<'a>>) {
        let server = self.clone();
        let path = path.into().into_owned();
        IoTaskPool::get()
            .spawn(async move {
                let mut reloaded = false;

                let requests = server
                    .data
                    .infos
                    .read()
                    .get_path_handles(&path)
                    .map(|handle| server.load_internal(Some(handle), path.clone(), true, None))
                    .collect::<Vec<_>>();

                for result in requests {
                    match result.await {
                        Ok(_) => reloaded = true,
                        Err(err) => error!("{}", err),
                    }
                }

                if !reloaded
                    && server.data.infos.read().should_reload(&path)
                    && let Err(err) = server.load_internal(None, path, true, None).await
                {
                    error!("{}", err);
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
        self.load_asset(LoadedAsset::new_with_dependencies(asset))
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
            let (handle, _) = self.data.infos.write().get_or_create_path_handle_erased(
                path,
                loaded_asset.asset_type_id(),
                Some(loaded_asset.asset_type_name()),
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

    /// Queues a new asset to be tracked by the [`AssetServer`] and returns a [`Handle`] to it. This can be used to track
    /// dependencies of assets created at runtime.
    ///
    /// After the asset has been fully loaded, it will show up in the relevant [`Assets`] storage.
    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn add_async<A: Asset, E: core::error::Error + Send + Sync + 'static>(
        &self,
        future: impl Future<Output = Result<A, E>> + Send + 'static,
    ) -> Handle<A> {
        let mut infos = self.data.infos.write();
        let handle =
            infos.create_loading_handle_untyped(TypeId::of::<A>(), core::any::type_name::<A>());

        // drop the lock on `AssetInfos` before spawning a task that may block on it in single-threaded
        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        drop(infos);

        let id = handle.id();

        let event_sender = self.data.asset_event_sender.clone();

        let task = IoTaskPool::get().spawn(async move {
            match future.await {
                Ok(asset) => {
                    let loaded_asset = LoadedAsset::new_with_dependencies(asset).into();
                    event_sender
                        .send(InternalAssetEvent::Loaded { id, loaded_asset })
                        .unwrap();
                }
                Err(error) => {
                    let error = AddAsyncError {
                        error: Arc::new(error),
                    };
                    error!("{error}");
                    event_sender
                        .send(InternalAssetEvent::Failed {
                            id,
                            path: Default::default(),
                            error: AssetLoadError::AddAsyncError(error),
                        })
                        .unwrap();
                }
            }
        });

        #[cfg(not(any(target_arch = "wasm32", not(feature = "multi_threaded"))))]
        infos.pending_tasks.insert(id, task);

        #[cfg(any(target_arch = "wasm32", not(feature = "multi_threaded")))]
        task.detach();

        handle.typed_debug_checked()
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
        async fn load_folder<'a>(
            source: AssetSourceId<'static>,
            path: &'a Path,
            reader: &'a dyn ErasedAssetReader,
            server: &'a AssetServer,
            handles: &'a mut Vec<UntypedHandle>,
        ) -> Result<(), AssetLoadError> {
            let is_dir = reader.is_directory(path).await?;
            if is_dir {
                let mut path_stream = reader.read_directory(path.as_ref()).await?;
                while let Some(child_path) = path_stream.next().await {
                    if reader.is_directory(&child_path).await? {
                        Box::pin(load_folder(
                            source.clone(),
                            &child_path,
                            reader,
                            server,
                            handles,
                        ))
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
        }

        let path = path.into_owned();
        let server = self.clone();
        IoTaskPool::get()
            .spawn(async move {
                let Ok(source) = server.get_source(path.source()) else {
                    error!(
                        "Failed to load {path}. AssetSource {} does not exist",
                        path.source()
                    );
                    return;
                };

                let asset_reader = match server.data.mode {
                    AssetServerMode::Unprocessed => source.reader(),
                    AssetServerMode::Processed => match source.processed_reader() {
                        Ok(reader) => reader,
                        Err(_) => {
                            error!(
                                "Failed to load {path}. AssetSource {} does not have a processed AssetReader",
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
                        )
                        .into(),
                    }),
                    Err(err) => {
                        error!("Failed to load folder. {err}");
                        server.send_asset_event(InternalAssetEvent::Failed { id, error: err, path });
                    },
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
        self.data.infos.read().get(id.into()).map(|i| {
            (
                i.load_state.clone(),
                i.dep_load_state.clone(),
                i.rec_dep_load_state.clone(),
            )
        })
    }

    /// Retrieves the main [`LoadState`] of a given asset `id`.
    ///
    /// Note that this is "just" the root asset load state. To get the load state of
    /// its dependencies or recursive dependencies, see [`AssetServer::get_dependency_load_state`]
    /// and [`AssetServer::get_recursive_dependency_load_state`] respectively.
    pub fn get_load_state(&self, id: impl Into<UntypedAssetId>) -> Option<LoadState> {
        self.data
            .infos
            .read()
            .get(id.into())
            .map(|i| i.load_state.clone())
    }

    /// Retrieves the [`DependencyLoadState`] of a given asset `id`'s dependencies.
    ///
    /// Note that this is only the load state of direct dependencies of the root asset. To get
    /// the load state of the root asset itself or its recursive dependencies, see
    /// [`AssetServer::get_load_state`] and [`AssetServer::get_recursive_dependency_load_state`] respectively.
    pub fn get_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<DependencyLoadState> {
        self.data
            .infos
            .read()
            .get(id.into())
            .map(|i| i.dep_load_state.clone())
    }

    /// Retrieves the main [`RecursiveDependencyLoadState`] of a given asset `id`'s recursive dependencies.
    ///
    /// Note that this is only the load state of recursive dependencies of the root asset. To get
    /// the load state of the root asset itself or its direct dependencies only, see
    /// [`AssetServer::get_load_state`] and [`AssetServer::get_dependency_load_state`] respectively.
    pub fn get_recursive_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Option<RecursiveDependencyLoadState> {
        self.data
            .infos
            .read()
            .get(id.into())
            .map(|i| i.rec_dep_load_state.clone())
    }

    /// Retrieves the main [`LoadState`] of a given asset `id`.
    ///
    /// This is the same as [`AssetServer::get_load_state`] except the result is unwrapped. If
    /// the result is None, [`LoadState::NotLoaded`] is returned.
    pub fn load_state(&self, id: impl Into<UntypedAssetId>) -> LoadState {
        self.get_load_state(id).unwrap_or(LoadState::NotLoaded)
    }

    /// Retrieves the [`DependencyLoadState`] of a given asset `id`.
    ///
    /// This is the same as [`AssetServer::get_dependency_load_state`] except the result is unwrapped. If
    /// the result is None, [`DependencyLoadState::NotLoaded`] is returned.
    pub fn dependency_load_state(&self, id: impl Into<UntypedAssetId>) -> DependencyLoadState {
        self.get_dependency_load_state(id)
            .unwrap_or(DependencyLoadState::NotLoaded)
    }

    /// Retrieves the  [`RecursiveDependencyLoadState`] of a given asset `id`.
    ///
    /// This is the same as [`AssetServer::get_recursive_dependency_load_state`] except the result is unwrapped. If
    /// the result is None, [`RecursiveDependencyLoadState::NotLoaded`] is returned.
    pub fn recursive_dependency_load_state(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> RecursiveDependencyLoadState {
        self.get_recursive_dependency_load_state(id)
            .unwrap_or(RecursiveDependencyLoadState::NotLoaded)
    }

    /// Convenience method that returns true if the asset has been loaded.
    pub fn is_loaded(&self, id: impl Into<UntypedAssetId>) -> bool {
        matches!(self.load_state(id), LoadState::Loaded)
    }

    /// Convenience method that returns true if the asset and all of its direct dependencies have been loaded.
    pub fn is_loaded_with_direct_dependencies(&self, id: impl Into<UntypedAssetId>) -> bool {
        matches!(
            self.get_load_states(id),
            Some((LoadState::Loaded, DependencyLoadState::Loaded, _))
        )
    }

    /// Convenience method that returns true if the asset, all of its dependencies, and all of its recursive
    /// dependencies have been loaded.
    pub fn is_loaded_with_dependencies(&self, id: impl Into<UntypedAssetId>) -> bool {
        matches!(
            self.get_load_states(id),
            Some((
                LoadState::Loaded,
                DependencyLoadState::Loaded,
                RecursiveDependencyLoadState::Loaded
            ))
        )
    }

    /// Returns an active handle for the given path, if the asset at the given path has already started loading,
    /// or is still "alive".
    pub fn get_handle<'a, A: Asset>(&self, path: impl Into<AssetPath<'a>>) -> Option<Handle<A>> {
        self.get_path_and_type_id_handle(&path.into(), TypeId::of::<A>())
            .map(UntypedHandle::typed_debug_checked)
    }

    /// Get a `Handle` from an `AssetId`.
    ///
    /// This only returns `Some` if `id` is derived from a `Handle` that was
    /// loaded through an `AssetServer`, otherwise it returns `None`.
    ///
    /// Consider using [`Assets::get_strong_handle`] in the case the `Handle`
    /// comes from [`Assets::add`].
    pub fn get_id_handle<A: Asset>(&self, id: AssetId<A>) -> Option<Handle<A>> {
        self.get_id_handle_untyped(id.untyped())
            .map(UntypedHandle::typed)
    }

    /// Get an `UntypedHandle` from an `UntypedAssetId`.
    /// See [`AssetServer::get_id_handle`] for details.
    pub fn get_id_handle_untyped(&self, id: UntypedAssetId) -> Option<UntypedHandle> {
        self.data.infos.read().get_id_handle(id)
    }

    /// Returns `true` if the given `id` corresponds to an asset that is managed by this [`AssetServer`].
    /// Otherwise, returns `false`.
    pub fn is_managed(&self, id: impl Into<UntypedAssetId>) -> bool {
        self.data.infos.read().contains_key(id.into())
    }

    /// Returns an active untyped asset id for the given path, if the asset at the given path has already started loading,
    /// or is still "alive".
    /// Returns the first ID in the event of multiple assets being registered against a single path.
    ///
    /// # See also
    /// [`get_path_ids`][Self::get_path_ids] for all handles.
    pub fn get_path_id<'a>(&self, path: impl Into<AssetPath<'a>>) -> Option<UntypedAssetId> {
        let infos = self.data.infos.read();
        let path = path.into();
        let mut ids = infos.get_path_ids(&path);
        ids.next()
    }

    /// Returns all active untyped asset IDs for the given path, if the assets at the given path have already started loading,
    /// or are still "alive".
    /// Multiple IDs will be returned in the event that a single path is used by multiple [`AssetLoader`]'s.
    pub fn get_path_ids<'a>(&self, path: impl Into<AssetPath<'a>>) -> Vec<UntypedAssetId> {
        let infos = self.data.infos.read();
        let path = path.into();
        infos.get_path_ids(&path).collect()
    }

    /// Returns an active untyped handle for the given path, if the asset at the given path has already started loading,
    /// or is still "alive".
    /// Returns the first handle in the event of multiple assets being registered against a single path.
    ///
    /// # See also
    /// [`get_handles_untyped`][Self::get_handles_untyped] for all handles.
    pub fn get_handle_untyped<'a>(&self, path: impl Into<AssetPath<'a>>) -> Option<UntypedHandle> {
        let infos = self.data.infos.read();
        let path = path.into();
        let mut handles = infos.get_path_handles(&path);
        handles.next()
    }

    /// Returns all active untyped handles for the given path, if the assets at the given path have already started loading,
    /// or are still "alive".
    /// Multiple handles will be returned in the event that a single path is used by multiple [`AssetLoader`]'s.
    pub fn get_handles_untyped<'a>(&self, path: impl Into<AssetPath<'a>>) -> Vec<UntypedHandle> {
        let infos = self.data.infos.read();
        let path = path.into();
        infos.get_path_handles(&path).collect()
    }

    /// Returns an active untyped handle for the given path and [`TypeId`], if the asset at the given path has already started loading,
    /// or is still "alive".
    pub fn get_path_and_type_id_handle(
        &self,
        path: &AssetPath,
        type_id: TypeId,
    ) -> Option<UntypedHandle> {
        let infos = self.data.infos.read();
        let path = path.into();
        infos.get_path_and_type_id_handle(&path, type_id)
    }

    /// Returns the path for the given `id`, if it has one.
    pub fn get_path(&self, id: impl Into<UntypedAssetId>) -> Option<AssetPath<'_>> {
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
        self.data.loaders.write().reserve::<L>(extensions);
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

    /// Retrieve a handle for the given path, where the asset type ID and name
    /// are not known statically.
    ///
    /// This will create a handle (and [`AssetInfo`]) if it does not exist.
    pub(crate) fn get_or_create_path_handle_erased<'a>(
        &self,
        path: impl Into<AssetPath<'a>>,
        type_id: TypeId,
        meta_transform: Option<MetaTransform>,
    ) -> UntypedHandle {
        let mut infos = self.data.infos.write();
        infos
            .get_or_create_path_handle_erased(
                path.into().into_owned(),
                type_id,
                None,
                HandleLoadingMode::NotLoading,
                meta_transform,
            )
            .0
    }

    pub(crate) async fn get_meta_loader_and_reader<'a>(
        &'a self,
        asset_path: &'a AssetPath<'_>,
        asset_type_id: Option<TypeId>,
    ) -> Result<
        (
            Box<dyn AssetMetaDyn>,
            Arc<dyn ErasedAssetLoader>,
            Box<dyn Reader + 'a>,
        ),
        AssetLoadError,
    > {
        let source = self.get_source(asset_path.source())?;
        // NOTE: We grab the asset byte reader first to ensure this is transactional for AssetReaders like ProcessorGatedReader
        // The asset byte reader will "lock" the processed asset, preventing writes for the duration of the lock.
        // Then the meta reader, if meta exists, will correspond to the meta for the current "version" of the asset.
        // See ProcessedAssetInfo::file_transaction_lock for more context
        let asset_reader = match self.data.mode {
            AssetServerMode::Unprocessed => source.reader(),
            AssetServerMode::Processed => source.processed_reader()?,
        };
        let reader = asset_reader.read(asset_path.path()).await?;
        let read_meta = match &self.data.meta_check {
            AssetMetaCheck::Always => true,
            AssetMetaCheck::Paths(paths) => paths.contains(asset_path),
            AssetMetaCheck::Never => false,
        };

        if read_meta {
            match asset_reader.read_meta_bytes(asset_path.path()).await {
                Ok(meta_bytes) => {
                    // TODO: this isn't fully minimal yet. we only need the loader
                    let minimal: AssetMetaMinimal =
                        ron::de::from_bytes(&meta_bytes).map_err(|e| {
                            AssetLoadError::DeserializeMeta {
                                path: asset_path.clone_owned(),
                                error: DeserializeMetaError::DeserializeMinimal(e).into(),
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
                            error: e.into(),
                        }
                    })?;

                    Ok((meta, loader, reader))
                }
                Err(AssetReaderError::NotFound(_)) => {
                    // TODO: Handle error transformation
                    let loader = {
                        self.data
                            .loaders
                            .read()
                            .find(None, asset_type_id, None, Some(asset_path))
                    };

                    let error = || AssetLoadError::MissingAssetLoader {
                        loader_name: None,
                        asset_type_id,
                        extension: None,
                        asset_path: Some(asset_path.to_string()),
                    };

                    let loader = loader.ok_or_else(error)?.get().await.map_err(|_| error())?;

                    let meta = loader.default_meta();
                    Ok((meta, loader, reader))
                }
                Err(err) => Err(err.into()),
            }
        } else {
            let loader = {
                self.data
                    .loaders
                    .read()
                    .find(None, asset_type_id, None, Some(asset_path))
            };

            let error = || AssetLoadError::MissingAssetLoader {
                loader_name: None,
                asset_type_id,
                extension: None,
                asset_path: Some(asset_path.to_string()),
            };

            let loader = loader.ok_or_else(error)?.get().await.map_err(|_| error())?;

            let meta = loader.default_meta();
            Ok((meta, loader, reader))
        }
    }

    pub(crate) async fn load_with_meta_loader_and_reader(
        &self,
        asset_path: &AssetPath<'_>,
        meta: &dyn AssetMetaDyn,
        loader: &dyn ErasedAssetLoader,
        reader: &mut dyn Reader,
        load_dependencies: bool,
        populate_hashes: bool,
    ) -> Result<ErasedLoadedAsset, AssetLoadError> {
        // TODO: experiment with this
        let asset_path = asset_path.clone_owned();
        let load_context =
            LoadContext::new(self, asset_path.clone(), load_dependencies, populate_hashes);
        AssertUnwindSafe(loader.load(reader, meta, load_context))
            .catch_unwind()
            .await
            .map_err(|_| AssetLoadError::AssetLoaderPanic {
                path: asset_path.clone_owned(),
                loader_name: loader.type_name(),
            })?
            .map_err(|e| {
                AssetLoadError::AssetLoaderError(AssetLoaderError {
                    path: asset_path.clone_owned(),
                    loader_name: loader.type_name(),
                    error: e.into(),
                })
            })
    }

    /// Returns a future that will suspend until the specified asset and its dependencies finish
    /// loading.
    ///
    /// # Errors
    ///
    /// This will return an error if the asset or any of its dependencies fail to load,
    /// or if the asset has not been queued up to be loaded.
    pub async fn wait_for_asset<A: Asset>(
        &self,
        // NOTE: We take a reference to a handle so we know it will outlive the future,
        // which ensures the handle won't be dropped while waiting for the asset.
        handle: &Handle<A>,
    ) -> Result<(), WaitForAssetError> {
        self.wait_for_asset_id(handle.id().untyped()).await
    }

    /// Returns a future that will suspend until the specified asset and its dependencies finish
    /// loading.
    ///
    /// # Errors
    ///
    /// This will return an error if the asset or any of its dependencies fail to load,
    /// or if the asset has not been queued up to be loaded.
    pub async fn wait_for_asset_untyped(
        &self,
        // NOTE: We take a reference to a handle so we know it will outlive the future,
        // which ensures the handle won't be dropped while waiting for the asset.
        handle: &UntypedHandle,
    ) -> Result<(), WaitForAssetError> {
        self.wait_for_asset_id(handle.id()).await
    }

    /// Returns a future that will suspend until the specified asset and its dependencies finish
    /// loading.
    ///
    /// Note that since an asset ID does not count as a reference to the asset,
    /// the future returned from this method will *not* keep the asset alive.
    /// This may lead to the asset unexpectedly being dropped while you are waiting for it to
    /// finish loading.
    ///
    /// When calling this method, make sure a strong handle is stored elsewhere to prevent the
    /// asset from being dropped.
    /// If you have access to an asset's strong [`Handle`], you should prefer to call
    /// [`AssetServer::wait_for_asset`]
    /// or [`wait_for_asset_untyped`](Self::wait_for_asset_untyped) to ensure the asset finishes
    /// loading.
    ///
    /// # Errors
    ///
    /// This will return an error if the asset or any of its dependencies fail to load,
    /// or if the asset has not been queued up to be loaded.
    pub async fn wait_for_asset_id(
        &self,
        id: impl Into<UntypedAssetId>,
    ) -> Result<(), WaitForAssetError> {
        let id = id.into();
        core::future::poll_fn(move |cx| self.wait_for_asset_id_poll_fn(cx, id)).await
    }

    /// Used by [`wait_for_asset_id`](AssetServer::wait_for_asset_id) in [`poll_fn`](core::future::poll_fn).
    fn wait_for_asset_id_poll_fn(
        &self,
        cx: &mut core::task::Context<'_>,
        id: UntypedAssetId,
    ) -> Poll<Result<(), WaitForAssetError>> {
        let infos = self.data.infos.read();

        let Some(info) = infos.get(id) else {
            return Poll::Ready(Err(WaitForAssetError::NotLoaded));
        };

        match (&info.load_state, &info.rec_dep_load_state) {
            (LoadState::Loaded, RecursiveDependencyLoadState::Loaded) => Poll::Ready(Ok(())),
            // Return an error immediately if the asset is not in the process of loading
            (LoadState::NotLoaded, _) => Poll::Ready(Err(WaitForAssetError::NotLoaded)),
            // If the asset is loading, leave our waker behind
            (LoadState::Loading, _)
            | (_, RecursiveDependencyLoadState::Loading)
            | (LoadState::Loaded, RecursiveDependencyLoadState::NotLoaded) => {
                // Check if our waker is already there
                let has_waker = info
                    .waiting_tasks
                    .iter()
                    .any(|waker| waker.will_wake(cx.waker()));

                if has_waker {
                    return Poll::Pending;
                }

                let mut infos = {
                    // Must drop read-only guard to acquire write guard
                    drop(infos);
                    self.data.infos.write()
                };

                let Some(info) = infos.get_mut(id) else {
                    return Poll::Ready(Err(WaitForAssetError::NotLoaded));
                };

                // If the load state changed while reacquiring the lock, immediately
                // reawaken the task
                let is_loading = matches!(
                    (&info.load_state, &info.rec_dep_load_state),
                    (LoadState::Loading, _)
                        | (_, RecursiveDependencyLoadState::Loading)
                        | (LoadState::Loaded, RecursiveDependencyLoadState::NotLoaded)
                );

                if !is_loading {
                    cx.waker().wake_by_ref();
                } else {
                    // Leave our waker behind
                    info.waiting_tasks.push(cx.waker().clone());
                }

                Poll::Pending
            }
            (LoadState::Failed(error), _) => {
                Poll::Ready(Err(WaitForAssetError::Failed(error.clone())))
            }
            (_, RecursiveDependencyLoadState::Failed(error)) => {
                Poll::Ready(Err(WaitForAssetError::DependencyFailed(error.clone())))
            }
        }
    }

    /// Writes the default loader meta file for the provided `path`.
    ///
    /// This function only generates meta files that simply load the path directly. To generate a
    /// meta file that will use the default asset processor for the path, see
    /// [`AssetProcessor::write_default_meta_file_for_path`].
    ///
    /// Note if there is already a meta file for `path`, this function returns
    /// `Err(WriteDefaultMetaError::MetaAlreadyExists)`.
    ///
    /// [`AssetProcessor::write_default_meta_file_for_path`]:  crate::AssetProcessor::write_default_meta_file_for_path
    pub async fn write_default_loader_meta_file_for_path(
        &self,
        path: impl Into<AssetPath<'_>>,
    ) -> Result<(), WriteDefaultMetaError> {
        let path = path.into();
        let loader = self.get_path_asset_loader(&path).await?;

        let meta = loader.default_meta();
        let serialized_meta = meta.serialize();

        let source = self.get_source(path.source())?;

        let reader = source.reader();
        match reader.read_meta_bytes(path.path()).await {
            Ok(_) => return Err(WriteDefaultMetaError::MetaAlreadyExists),
            Err(AssetReaderError::NotFound(_)) => {
                // The meta file couldn't be found so just fall through.
            }
            Err(AssetReaderError::Io(err)) => {
                return Err(WriteDefaultMetaError::IoErrorFromExistingMetaCheck(err))
            }
            Err(AssetReaderError::HttpError(err)) => {
                return Err(WriteDefaultMetaError::HttpErrorFromExistingMetaCheck(err))
            }
        }

        let writer = source.writer()?;
        writer
            .write_meta_bytes(path.path(), &serialized_meta)
            .await?;

        Ok(())
    }
}

/// A system that manages internal [`AssetServer`] events, such as finalizing asset loads.
pub fn handle_internal_asset_events(world: &mut World) {
    world.resource_scope(|world, server: Mut<AssetServer>| {
        let mut infos = server.data.infos.write();
        let var_name = vec![];
        let mut untyped_failures = var_name;
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
                    if let Some(info) = infos.get_mut(id) {
                        for waker in info.waiting_tasks.drain(..) {
                            waker.wake();
                        }
                    }
                }
                InternalAssetEvent::Failed { id, path, error } => {
                    infos.process_asset_fail(id, error.clone());

                    // Send untyped failure event
                    untyped_failures.push(UntypedAssetLoadFailedEvent {
                        id,
                        path: path.clone(),
                        error: error.clone(),
                    });

                    // Send typed failure event
                    let sender = infos
                        .dependency_failed_event_sender
                        .get(&id.type_id())
                        .expect("Asset failed event sender should exist");
                    sender(world, id, path, error);
                }
            }
        }

        if !untyped_failures.is_empty() {
            world.write_event_batch(untyped_failures);
        }

        fn queue_ancestors(
            asset_path: &AssetPath,
            infos: &AssetInfos,
            paths_to_reload: &mut HashSet<AssetPath<'static>>,
        ) {
            if let Some(dependents) = infos.loader_dependents.get(asset_path) {
                for dependent in dependents {
                    paths_to_reload.insert(dependent.to_owned());
                    queue_ancestors(dependent, infos, paths_to_reload);
                }
            }
        }

        let reload_parent_folders = |path: PathBuf, source: &AssetSourceId<'static>| {
            let mut current_folder = path;
            while let Some(parent) = current_folder.parent() {
                current_folder = parent.to_path_buf();
                let parent_asset_path =
                    AssetPath::from(current_folder.clone()).with_source(source.clone());
                for folder_handle in infos.get_path_handles(&parent_asset_path) {
                    info!("Reloading folder {parent_asset_path} because the content has changed");
                    server.load_folder_internal(folder_handle.id(), parent_asset_path.clone());
                }
            }
        };

        let mut paths_to_reload = <HashSet<_>>::default();
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
                AssetServerMode::Unprocessed => {
                    if let Some(receiver) = source.event_receiver() {
                        for event in receiver.try_iter() {
                            handle_event(source.id(), event);
                        }
                    }
                }
                AssetServerMode::Processed => {
                    if let Some(receiver) = source.processed_event_receiver() {
                        for event in receiver.try_iter() {
                            handle_event(source.id(), event);
                        }
                    }
                }
            }
        }

        for path in paths_to_reload {
            info!("Reloading {path} because it has changed");
            server.reload(path);
        }

        #[cfg(not(any(target_arch = "wasm32", not(feature = "multi_threaded"))))]
        infos
            .pending_tasks
            .retain(|_, load_task| !load_task.is_finished());
    });
}

/// Internal events for asset load results
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
        path: AssetPath<'static>,
        error: AssetLoadError,
    },
}

/// The load state of an asset.
#[derive(Component, Clone, Debug)]
pub enum LoadState {
    /// The asset has not started loading yet
    NotLoaded,

    /// The asset is in the process of loading.
    Loading,

    /// The asset has been loaded and has been added to the [`World`]
    Loaded,

    /// The asset failed to load. The underlying [`AssetLoadError`] is
    /// referenced by [`Arc`] clones in all related [`DependencyLoadState`]s
    /// and [`RecursiveDependencyLoadState`]s in the asset's dependency tree.
    Failed(Arc<AssetLoadError>),
}

impl LoadState {
    /// Returns `true` if this instance is [`LoadState::Loading`]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns `true` if this instance is [`LoadState::Loaded`]
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    /// Returns `true` if this instance is [`LoadState::Failed`]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// The load state of an asset's dependencies.
#[derive(Component, Clone, Debug)]
pub enum DependencyLoadState {
    /// The asset has not started loading yet
    NotLoaded,

    /// Dependencies are still loading
    Loading,

    /// Dependencies have all loaded
    Loaded,

    /// One or more dependencies have failed to load. The underlying [`AssetLoadError`]
    /// is referenced by [`Arc`] clones in all related [`LoadState`] and
    /// [`RecursiveDependencyLoadState`]s in the asset's dependency tree.
    Failed(Arc<AssetLoadError>),
}

impl DependencyLoadState {
    /// Returns `true` if this instance is [`DependencyLoadState::Loading`]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns `true` if this instance is [`DependencyLoadState::Loaded`]
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    /// Returns `true` if this instance is [`DependencyLoadState::Failed`]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// The recursive load state of an asset's dependencies.
#[derive(Component, Clone, Debug)]
pub enum RecursiveDependencyLoadState {
    /// The asset has not started loading yet
    NotLoaded,

    /// Dependencies in this asset's dependency tree are still loading
    Loading,

    /// Dependencies in this asset's dependency tree have all loaded
    Loaded,

    /// One or more dependencies have failed to load in this asset's dependency
    /// tree. The underlying [`AssetLoadError`] is referenced by [`Arc`] clones
    /// in all related [`LoadState`]s and [`DependencyLoadState`]s in the asset's
    /// dependency tree.
    Failed(Arc<AssetLoadError>),
}

impl RecursiveDependencyLoadState {
    /// Returns `true` if this instance is [`RecursiveDependencyLoadState::Loading`]
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    /// Returns `true` if this instance is [`RecursiveDependencyLoadState::Loaded`]
    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded)
    }

    /// Returns `true` if this instance is [`RecursiveDependencyLoadState::Failed`]
    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failed(_))
    }
}

/// An error that occurs during an [`Asset`] load.
#[derive(Error, Debug, Clone)]
#[expect(
    missing_docs,
    reason = "Adding docs to the variants would not add information beyond the error message and the names"
)]
pub enum AssetLoadError {
    #[error("Requested handle of type {requested:?} for asset '{path}' does not match actual asset type '{actual_asset_name}', which used loader '{loader_name}'")]
    RequestedHandleTypeMismatch {
        path: AssetPath<'static>,
        requested: TypeId,
        actual_asset_name: &'static str,
        loader_name: &'static str,
    },
    #[error("Could not find an asset loader matching: Loader Name: {loader_name:?}; Asset Type: {loader_name:?}; Extension: {extension:?}; Path: {asset_path:?};")]
    MissingAssetLoader {
        loader_name: Option<String>,
        asset_type_id: Option<TypeId>,
        extension: Option<String>,
        asset_path: Option<String>,
    },
    #[error(transparent)]
    MissingAssetLoaderForExtension(#[from] MissingAssetLoaderForExtensionError),
    #[error(transparent)]
    MissingAssetLoaderForTypeName(#[from] MissingAssetLoaderForTypeNameError),
    #[error(transparent)]
    MissingAssetLoaderForTypeIdError(#[from] MissingAssetLoaderForTypeIdError),
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
    #[from(ignore)]
    CannotLoadProcessedAsset { path: AssetPath<'static> },
    #[error("Asset '{path}' is configured to be ignored. It cannot be loaded.")]
    #[from(ignore)]
    CannotLoadIgnoredAsset { path: AssetPath<'static> },
    #[error("Failed to load asset '{path}', asset loader '{loader_name}' panicked")]
    AssetLoaderPanic {
        path: AssetPath<'static>,
        loader_name: &'static str,
    },
    #[error(transparent)]
    AssetLoaderError(#[from] AssetLoaderError),
    #[error(transparent)]
    AddAsyncError(#[from] AddAsyncError),
    #[error("The file at '{}' does not contain the labeled asset '{}'; it contains the following {} assets: {}",
            base_path,
            label,
            all_labels.len(),
            all_labels.iter().map(|l| format!("'{l}'")).collect::<Vec<_>>().join(", "))]
    MissingLabel {
        base_path: AssetPath<'static>,
        label: String,
        all_labels: Vec<String>,
    },
}

/// An error that can occur during asset loading.
#[derive(Error, Debug, Clone)]
#[error("Failed to load asset '{path}' with asset loader '{loader_name}': {error}")]
pub struct AssetLoaderError {
    path: AssetPath<'static>,
    loader_name: &'static str,
    error: Arc<BevyError>,
}

impl AssetLoaderError {
    /// The path of the asset that failed to load.
    pub fn path(&self) -> &AssetPath<'static> {
        &self.path
    }

    /// The error the loader reported when attempting to load the asset.
    ///
    /// If you know the type of the error the asset loader returned, you can use
    /// [`BevyError::downcast_ref()`] to get it.
    pub fn error(&self) -> &BevyError {
        &self.error
    }
}

/// An error that occurs while resolving an asset added by `add_async`.
#[derive(Error, Debug, Clone)]
#[error("An error occurred while resolving an asset added by `add_async`: {error}")]
pub struct AddAsyncError {
    error: Arc<dyn core::error::Error + Send + Sync + 'static>,
}

/// An error that occurs when an [`AssetLoader`] is not registered for a given extension.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("no `AssetLoader` found{}", format_missing_asset_ext(extensions))]
pub struct MissingAssetLoaderForExtensionError {
    extensions: Vec<String>,
}

/// An error that occurs when an [`AssetLoader`] is not registered for a given [`core::any::type_name`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("no `AssetLoader` found with the name '{type_name}'")]
pub struct MissingAssetLoaderForTypeNameError {
    /// The type name that was not found.
    pub type_name: String,
}

/// An error that occurs when an [`AssetLoader`] is not registered for a given [`Asset`] [`TypeId`].
#[derive(Error, Debug, Clone, PartialEq, Eq)]
#[error("no `AssetLoader` found with the ID '{type_id:?}'")]
pub struct MissingAssetLoaderForTypeIdError {
    /// The type ID that was not found.
    pub type_id: TypeId,
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

impl core::fmt::Debug for AssetServer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AssetServer")
            .field("info", &self.data.infos.read())
            .finish()
    }
}

/// This is appended to asset sources when loading a [`LoadedUntypedAsset`]. This provides a unique
/// source for a given [`AssetPath`].
const UNTYPED_SOURCE_SUFFIX: &str = "--untyped";

/// An error when attempting to wait asynchronously for an [`Asset`] to load.
#[derive(Error, Debug, Clone)]
pub enum WaitForAssetError {
    /// The asset is not being loaded; waiting for it is meaningless.
    #[error("tried to wait for an asset that is not being loaded")]
    NotLoaded,
    /// The asset failed to load.
    #[error(transparent)]
    Failed(Arc<AssetLoadError>),
    /// A dependency of the asset failed to load.
    #[error(transparent)]
    DependencyFailed(Arc<AssetLoadError>),
}

#[derive(Error, Debug)]
pub enum WriteDefaultMetaError {
    #[error(transparent)]
    MissingAssetLoader(#[from] MissingAssetLoaderForExtensionError),
    #[error(transparent)]
    MissingAssetSource(#[from] MissingAssetSourceError),
    #[error(transparent)]
    MissingAssetWriter(#[from] MissingAssetWriterError),
    #[error("failed to write default asset meta file: {0}")]
    FailedToWriteMeta(#[from] AssetWriterError),
    #[error("asset meta file already exists, so avoiding overwrite")]
    MetaAlreadyExists,
    #[error("encountered an I/O error while reading the existing meta file: {0}")]
    IoErrorFromExistingMetaCheck(Arc<std::io::Error>),
    #[error("encountered HTTP status {0} when reading the existing meta file")]
    HttpErrorFromExistingMetaCheck(u16),
}
