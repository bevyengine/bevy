use crate::{
    path::{AssetPath, AssetPathId, SourcePathId},
    Asset, AssetIo, AssetIoError, AssetLifecycle, AssetLifecycleChannel, AssetLifecycleEvent,
    AssetLoader, Assets, Handle, HandleId, HandleUntyped, LabelId, LoadContext, LoadState,
    RefChange, RefChangeChannel, SourceInfo, SourceMeta,
};
use anyhow::Result;
use bevy_ecs::Res;
use bevy_tasks::TaskPool;
use bevy_utils::{HashMap, Uuid};
use crossbeam_channel::TryRecvError;
use parking_lot::RwLock;
use std::{collections::hash_map::Entry, path::Path, sync::Arc};
use thiserror::Error;

/// Errors that occur while loading assets with an AssetServer
#[derive(Error, Debug)]
pub enum AssetServerError {
    #[error("asset folder path is not a directory")]
    AssetFolderNotADirectory(String),
    #[error("no AssetLoader found for the given extension")]
    MissingAssetLoader(Option<String>),
    #[error("the given type does not match the type of the loaded asset")]
    IncorrectHandleType,
    #[error("encountered an error while loading an asset")]
    AssetLoaderError(anyhow::Error),
    #[error("`PathLoader` encountered an error")]
    PathLoaderError(#[from] AssetIoError),
}

#[derive(Default)]
pub(crate) struct AssetRefCounter {
    pub(crate) channel: Arc<RefChangeChannel>,
    pub(crate) ref_counts: Arc<RwLock<HashMap<HandleId, usize>>>,
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
            .ok_or_else(|| AssetServerError::MissingAssetLoader(Some(extension.to_string())))
    }

    fn get_path_asset_loader<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Arc<Box<dyn AssetLoader>>, AssetServerError> {
        path.as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .ok_or(AssetServerError::MissingAssetLoader(None))
            .and_then(|extension| self.get_asset_loader(extension))
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
                },
                HandleId::Id(_, _) => return LoadState::NotLoaded,
            }
        }

        load_state
    }

    pub fn load<'a, T: Asset, P: Into<AssetPath<'a>>>(&self, path: P) -> Handle<T> {
        self.load_untyped(path).typed()
    }

    // TODO: properly set failed LoadState in all failure cases
    async fn load_async<'a, P: Into<AssetPath<'a>>>(
        &self,
        path: P,
        force: bool,
    ) -> Result<AssetPathId, AssetServerError> {
        let asset_path: AssetPath = path.into();
        let asset_loader = self.get_path_asset_loader(asset_path.path())?;
        let asset_path_id: AssetPathId = asset_path.get_id();

        // load metadata and update source info. this is done in a scope to ensure we release the locks before loading
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

            // if asset is already loaded (or is loading), don't load again
            if !force
                && source_info
                    .committed_assets
                    .contains(&asset_path_id.label_id())
            {
                return Ok(asset_path_id);
            }

            source_info.load_state = LoadState::Loading;
            source_info.committed_assets.clear();
            source_info.version += 1;
            source_info.meta = None;
            source_info.version
        };

        // load the asset bytes
        let bytes = self.server.asset_io.load_path(asset_path.path()).await?;

        // load the asset source using the corresponding AssetLoader
        let mut load_context = LoadContext::new(
            asset_path.path(),
            &self.server.asset_ref_counter.channel,
            &*self.server.asset_io,
            version,
        );
        asset_loader
            .load(&bytes, &mut load_context)
            .await
            .map_err(AssetServerError::AssetLoaderError)?;

        // if version has changed since we loaded and grabbed a lock, return. theres is a newer version being loaded
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
                self.load_untyped(dependency.clone());
            }
        }

        self.server
            .asset_io
            .watch_path_for_changes(asset_path.path())
            .unwrap();
        self.create_assets_in_load_context(&mut load_context);
        Ok(asset_path_id)
    }

    pub fn load_untyped<'a, P: Into<AssetPath<'a>>>(&self, path: P) -> HandleUntyped {
        let handle_id = self.load_untracked(path, false);
        self.get_handle_untyped(handle_id)
    }

    pub(crate) fn load_untracked<'a, P: Into<AssetPath<'a>>>(
        &self,
        path: P,
        force: bool,
    ) -> HandleId {
        let asset_path: AssetPath<'a> = path.into();
        let server = self.clone();
        let owned_path = asset_path.to_owned();
        self.server
            .task_pool
            .spawn(async move {
                server.load_async(owned_path, force).await.unwrap();
            })
            .detach();
        asset_path.into()
    }

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
        let receiver = &self.server.asset_ref_counter.channel.receiver;
        let mut ref_counts = self.server.asset_ref_counter.ref_counts.write();
        let asset_sources = self.server.asset_sources.read();
        let mut potential_frees = Vec::new();
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
                        potential_frees.push(handle_id);
                    }
                }
            }
        }

        if !potential_frees.is_empty() {
            let asset_lifecycles = self.server.asset_lifecycles.read();
            for potential_free in potential_frees {
                if let Some(i) = ref_counts.get(&potential_free).cloned() {
                    if i == 0 {
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
                panic!("Failed to find AssetLifecycle for label {:?}, which has an asset type {:?}. Are you sure that is a registered asset type?", label, asset_value.type_uuid());
            }
        }
    }

    pub(crate) fn update_asset_storage<T: Asset>(&self, assets: &mut Assets<T>) {
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

                    assets.set(result.id, result.asset);
                }
                Ok(AssetLifecycleEvent::Free(handle_id)) => {
                    if let HandleId::AssetPathId(id) = handle_id {
                        let asset_sources = asset_sources_guard
                            .get_or_insert_with(|| self.server.asset_sources.write());
                        if let Some(source_info) = asset_sources.get_mut(&id.source_path_id()) {
                            source_info.committed_assets.remove(&id.label_id());
                            if source_info.is_loaded() {
                                source_info.load_state = LoadState::Loaded;
                            }
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

pub fn free_unused_assets_system(asset_server: Res<AssetServer>) {
    asset_server.free_unused_assets();
}
