use crate::{
    path::{AssetPath, AssetPathId, SourcePathId},
    Asset, AssetIo, AssetIoError, AssetLifecycle, AssetLifecycleChannel, AssetLifecycleEvent,
    AssetLoader, Assets, Handle, HandleId, HandleUntyped, LabelId, LoadContext, LoadState,
    RefChange, RefChangeChannel, SourceInfo, SourceMeta,
};
use anyhow::Result;
use bevy_ecs::system::{Res, ResMut};
use bevy_log::warn;
use bevy_tasks::TaskPool;
use bevy_utils::{HashMap, Uuid};
use crossbeam_channel::TryRecvError;
use parking_lot::{Mutex, RwLock};
use std::{collections::hash_map::Entry, path::Path, sync::Arc};
use thiserror::Error;

/// Errors that occur while loading assets with an AssetServer
#[derive(Error, Debug)]
pub enum AssetServerError {
    #[error("asset folder path is not a directory: {0}")]
    AssetFolderNotADirectory(String),
    #[error("no `AssetLoader` found{}", format_missing_asset_ext(.extensions))]
    MissingAssetLoader { extensions: Vec<String> },
    #[error("the given type does not match the type of the loaded asset")]
    IncorrectHandleType,
    #[error("encountered an error while loading an asset: {0}")]
    AssetLoaderError(anyhow::Error),
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

pub struct AssetServerInternal {
    pub(crate) asset_io: Box<dyn AssetIo>,
    pub(crate) asset_ref_counter: AssetRefCounter,
    pub(crate) asset_sources: Arc<RwLock<HashMap<SourcePathId, SourceInfo>>>,
    pub(crate) asset_lifecycles: Arc<RwLock<HashMap<Uuid, Box<dyn AssetLifecycle>>>>,
    loaders: RwLock<Vec<Arc<Box<dyn AssetLoader>>>>,
    extension_to_loader_index: RwLock<HashMap<String, usize>>,
    handle_to_path: Arc<RwLock<HashMap<HandleId, AssetPath<'static>>>>,
    task_pool: TaskPool,
}

/// Loads assets from the filesystem on background threads
pub struct AssetServer {
    pub(crate) server: Arc<AssetServerInternal>,
}

impl Clone for AssetServer {
    fn clone(&self) -> Self {
        Self {
            server: self.server.clone(),
        }
    }
}

impl AssetServer {
    pub fn new<T: AssetIo>(source_io: T, task_pool: TaskPool) -> Self {
        Self::with_boxed_io(Box::new(source_io), task_pool)
    }

    pub fn with_boxed_io(asset_io: Box<dyn AssetIo>, task_pool: TaskPool) -> Self {
        AssetServer {
            server: Arc::new(AssetServerInternal {
                loaders: Default::default(),
                extension_to_loader_index: Default::default(),
                asset_sources: Default::default(),
                asset_ref_counter: Default::default(),
                handle_to_path: Default::default(),
                asset_lifecycles: Default::default(),
                task_pool,
                asset_io,
            }),
        }
    }

    pub(crate) fn register_asset_type<T: Asset>(&self) -> Assets<T> {
        self.server.asset_lifecycles.write().insert(
            T::TYPE_UUID,
            Box::new(AssetLifecycleChannel::<T>::default()),
        );
        Assets::new(self.server.asset_ref_counter.channel.sender.clone())
    }

    pub fn add_loader<T>(&self, loader: T)
    where
        T: AssetLoader,
    {
        let mut loaders = self.server.loaders.write();
        let loader_index = loaders.len();
        for extension in loader.extensions().iter() {
            self.server
                .extension_to_loader_index
                .write()
                .insert(extension.to_string(), loader_index);
        }
        loaders.push(Arc::new(Box::new(loader)));
    }

    pub fn watch_for_changes(&self) -> Result<(), AssetServerError> {
        self.server.asset_io.watch_for_changes()?;
        Ok(())
    }

    pub fn get_handle<T: Asset, I: Into<HandleId>>(&self, id: I) -> Handle<T> {
        let sender = self.server.asset_ref_counter.channel.sender.clone();
        Handle::strong(id.into(), sender)
    }

    pub fn get_handle_untyped<I: Into<HandleId>>(&self, id: I) -> HandleUntyped {
        let sender = self.server.asset_ref_counter.channel.sender.clone();
        HandleUntyped::strong(id.into(), sender)
    }

    fn get_asset_loader(
        &self,
        extension: &str,
    ) -> Result<Arc<Box<dyn AssetLoader>>, AssetServerError> {
        self.server
            .extension_to_loader_index
            .read()
            .get(extension)
            .map(|index| self.server.loaders.read()[*index].clone())
            .ok_or_else(|| AssetServerError::MissingAssetLoader {
                extensions: vec![extension.to_string()],
            })
    }

    fn get_path_asset_loader<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Arc<Box<dyn AssetLoader>>, AssetServerError> {
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
                return Ok(loader);
            }
        }
        Err(AssetServerError::MissingAssetLoader {
            extensions: exts.into_iter().map(String::from).collect(),
        })
    }

    pub fn get_handle_path<H: Into<HandleId>>(&self, handle: H) -> Option<AssetPath<'_>> {
        self.server
            .handle_to_path
            .read()
            .get(&handle.into())
            .cloned()
    }

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

    /// Loads an Asset at the provided relative path.
    ///
    /// The absolute Path to the asset is "ROOT/ASSET_FOLDER_NAME/path".
    ///
    /// By default the ROOT is the directory of the Application, but this can be overridden by
    /// setting the `"CARGO_MANIFEST_DIR"` environment variable (see https://doc.rust-lang.org/cargo/reference/environment-variables.html)
    /// to another directory. When the application  is run through Cargo, then
    /// `"CARGO_MANIFEST_DIR"` is automatically set to the root folder of your crate (workspace).
    ///
    /// The name of the asset folder is set inside the
    /// [`AssetServerSettings`](crate::AssetServerSettings) resource. The default name is
    /// `"assets"`.
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
        let asset_loader = match self.get_path_asset_loader(asset_path.path()) {
            Ok(loader) => loader,
            Err(err) => {
                set_asset_failed();
                return Err(err);
            }
        };

        // load the asset bytes
        let bytes = match self.server.asset_io.load_path(asset_path.path()).await {
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
            &*self.server.asset_io,
            version,
            &self.server.task_pool,
        );

        if let Err(err) = asset_loader
            .load(&bytes, &mut load_context)
            .await
            .map_err(AssetServerError::AssetLoaderError)
        {
            set_asset_failed();
            return Err(err);
        }

        // if version has changed since we loaded and grabbed a lock, return. theres is a newer
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
        for (label, loaded_asset) in load_context.labeled_assets.iter_mut() {
            let label_id = LabelId::from(label.as_ref().map(|label| label.as_str()));
            let type_uuid = loaded_asset.value.as_ref().unwrap().type_uuid();
            source_info.asset_types.insert(label_id, type_uuid);
            for dependency in loaded_asset.dependencies.iter() {
                self.load_untracked(dependency.clone(), false);
            }
        }

        self.server
            .asset_io
            .watch_path_for_changes(asset_path.path())
            .unwrap();
        self.create_assets_in_load_context(&mut load_context);
        Ok(asset_path_id)
    }

    #[must_use = "not using the returned strong handle may result in the unexpected release of the asset"]
    pub fn load_untyped<'a, P: Into<AssetPath<'a>>>(&self, path: P) -> HandleUntyped {
        let handle_id = self.load_untracked(path.into(), false);
        self.get_handle_untyped(handle_id)
    }

    pub(crate) fn load_untracked(&self, asset_path: AssetPath<'_>, force: bool) -> HandleId {
        let server = self.clone();
        let owned_path = asset_path.to_owned();
        self.server
            .task_pool
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

    #[must_use = "not using the returned strong handles may result in the unexpected release of the assets"]
    pub fn load_folder<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<HandleUntyped>, AssetServerError> {
        let path = path.as_ref();
        if !self.server.asset_io.is_directory(path) {
            return Err(AssetServerError::AssetFolderNotADirectory(
                path.to_str().unwrap().to_string(),
            ));
        }

        let mut handles = Vec::new();
        for child_path in self.server.asset_io.read_directory(path.as_ref())? {
            if self.server.asset_io.is_directory(&child_path) {
                handles.extend(self.load_folder(&child_path)?);
            } else {
                if self.get_path_asset_loader(&child_path).is_err() {
                    continue;
                }
                let handle =
                    self.load_untyped(child_path.to_str().expect("Path should be a valid string."));
                handles.push(handle);
            }
        }

        Ok(handles)
    }

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
        for (label, asset) in load_context.labeled_assets.iter_mut() {
            let asset_value = asset
                .value
                .take()
                .expect("Asset should exist at this point.");
            if let Some(asset_lifecycle) = asset_lifecycles.get(&asset_value.type_uuid()) {
                let asset_path =
                    AssetPath::new_ref(&load_context.path, label.as_ref().map(|l| l.as_str()));
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

pub fn free_unused_assets_system(asset_server: Res<AssetServer>) {
    free_unused_assets_system_impl(&asset_server);
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{loader::LoadedAsset, update_asset_storage_system};
    use bevy_ecs::prelude::*;
    use bevy_reflect::TypeUuid;
    use bevy_utils::BoxedFuture;

    #[derive(Debug, TypeUuid)]
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

        AssetServer {
            server: Arc::new(AssetServerInternal {
                loaders: Default::default(),
                extension_to_loader_index: Default::default(),
                asset_sources: Default::default(),
                asset_ref_counter: Default::default(),
                handle_to_path: Default::default(),
                asset_lifecycles: Default::default(),
                task_pool: Default::default(),
                asset_io: Box::new(FileAssetIo::new(asset_path)),
            }),
        }
    }

    #[test]
    fn extensions() {
        let asset_server = setup(".");
        asset_server.add_loader(FakePngLoader);

        let t = asset_server.get_path_asset_loader("test.png");
        assert_eq!(t.unwrap().extensions()[0], "png");
    }

    #[test]
    fn case_insensitive_extensions() {
        let asset_server = setup(".");
        asset_server.add_loader(FakePngLoader);

        let t = asset_server.get_path_asset_loader("test.PNG");
        assert_eq!(t.unwrap().extensions()[0], "png");
    }

    #[test]
    fn no_loader() {
        let asset_server = setup(".");
        let t = asset_server.get_path_asset_loader("test.pong");
        assert!(t.is_err());
    }

    #[test]
    fn multiple_extensions_no_loader() {
        let asset_server = setup(".");

        assert!(
            match asset_server.get_path_asset_loader("test.v1.2.3.pong") {
                Err(AssetServerError::MissingAssetLoader { extensions }) =>
                    extensions == vec!["v1.2.3.pong", "2.3.pong", "3.pong", "pong"],
                _ => false,
            }
        )
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

        let t = asset_server.get_path_asset_loader("test-v1.2.3.png");
        assert_eq!(t.unwrap().extensions()[0], "png");
    }

    #[test]
    fn multiple_extensions() {
        let asset_server = setup(".");
        asset_server.add_loader(FakeMultipleDotLoader);

        let t = asset_server.get_path_asset_loader("test.test.png");
        assert_eq!(t.unwrap().extensions()[0], "test.png");
    }

    fn create_dir_and_file(file: impl AsRef<Path>) -> tempfile::TempDir {
        let asset_dir = tempfile::tempdir().unwrap();
        std::fs::write(asset_dir.path().join(file), &[]).unwrap();
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

        let mut world = World::new();
        world.insert_resource(assets);
        world.insert_resource(asset_server);

        let mut tick = {
            let mut free_unused_assets_system = free_unused_assets_system.system();
            free_unused_assets_system.initialize(&mut world);
            let mut update_asset_storage_system = update_asset_storage_system::<PngAsset>.system();
            update_asset_storage_system.initialize(&mut world);

            move |world: &mut World| {
                free_unused_assets_system.run((), world);
                update_asset_storage_system.run((), world);
            }
        };

        fn load_asset(path: AssetPath, world: &World) -> HandleUntyped {
            let asset_server = world.get_resource::<AssetServer>().unwrap();
            let id = futures_lite::future::block_on(asset_server.load_async(path.clone(), true))
                .unwrap();
            asset_server.get_handle_untyped(id)
        }

        fn get_asset(id: impl Into<HandleId>, world: &World) -> Option<&PngAsset> {
            world
                .get_resource::<Assets<PngAsset>>()
                .unwrap()
                .get(id.into())
        }

        fn get_load_state(id: impl Into<HandleId>, world: &World) -> LoadState {
            world
                .get_resource::<AssetServer>()
                .unwrap()
                .get_load_state(id.into())
        }

        // ---
        // Start of the actual lifecycle test
        // ---

        let path: AssetPath = "fake.png".into();
        assert_eq!(LoadState::NotLoaded, get_load_state(path.get_id(), &world));

        // load the asset
        let handle = load_asset(path.clone(), &world);
        let weak_handle = handle.clone_weak();

        // asset is loading
        assert_eq!(LoadState::Loading, get_load_state(&handle, &world));

        tick(&mut world);
        // asset should exist and be loaded at this point
        assert_eq!(LoadState::Loaded, get_load_state(&handle, &world));
        assert!(get_asset(&handle, &world).is_some());

        // after dropping the handle, next call to `tick` will prepare the assets for removal.
        drop(handle);
        tick(&mut world);
        assert_eq!(LoadState::Loaded, get_load_state(&weak_handle, &world));
        assert!(get_asset(&weak_handle, &world).is_some());

        // second call to tick will actually remove the asset.
        tick(&mut world);
        assert_eq!(LoadState::Unloaded, get_load_state(&weak_handle, &world));
        assert!(get_asset(&weak_handle, &world).is_none());

        // finally, reload the asset
        let handle = load_asset(path.clone(), &world);
        assert_eq!(LoadState::Loading, get_load_state(&handle, &world));
        tick(&mut world);
        assert_eq!(LoadState::Loaded, get_load_state(&handle, &world));
        assert!(get_asset(&handle, &world).is_some());
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
