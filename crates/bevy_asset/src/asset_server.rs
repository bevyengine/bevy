use crate::{
    filesystem_watcher::FilesystemWatcher, AssetLoadError, AssetLoadRequestHandler, AssetLoader,
    Assets, Handle, HandleId, LoadRequest,
};
use anyhow::Result;
use bevy_ecs::{Res, Resource, Resources};
use crossbeam_channel::TryRecvError;
use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
};
use thiserror::Error;

/// The type used for asset versioning
pub type AssetVersion = usize;

/// Errors that occur while loading assets with an AssetServer
#[derive(Error, Debug)]
pub enum AssetServerError {
    #[error("Asset folder path is not a directory.")]
    AssetFolderNotADirectory(String),
    #[error("Invalid root path")]
    InvalidRootPath,
    #[error("No AssetHandler found for the given extension.")]
    MissingAssetHandler,
    #[error("No AssetLoader found for the given extension.")]
    MissingAssetLoader,
    #[error("Encountered an error while loading an asset.")]
    AssetLoadError(#[from] AssetLoadError),
    #[error("Encountered an io error.")]
    Io(#[from] io::Error),
    #[error("Failed to watch asset folder.")]
    AssetWatchError { path: PathBuf },
}

struct LoaderThread {
    // NOTE: these must remain private. the LoaderThread Arc counters are used to determine thread liveness
    // if there is one reference, the loader thread is dead. if there are two references, the loader thread is active
    requests: Arc<RwLock<Vec<LoadRequest>>>,
}

/// Info about a specific asset, such as its path and its current load state
#[derive(Clone, Debug)]
pub struct AssetInfo {
    pub handle_id: HandleId,
    pub path: PathBuf,
    pub load_state: LoadState,
}

/// The load state of an asset
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadState {
    Loading(AssetVersion),
    Loaded(AssetVersion),
    Failed(AssetVersion),
}

impl LoadState {
    pub fn get_version(&self) -> AssetVersion {
        match *self {
            LoadState::Loaded(version) => version,
            LoadState::Loading(version) => version,
            LoadState::Failed(version) => version,
        }
    }
}

/// Loads assets from the filesystem on background threads
pub struct AssetServer {
    asset_folders: RwLock<Vec<PathBuf>>,
    loader_threads: RwLock<Vec<LoaderThread>>,
    max_loader_threads: usize,
    asset_handlers: Arc<RwLock<Vec<Box<dyn AssetLoadRequestHandler>>>>,
    // TODO: this is a hack to enable retrieving generic AssetLoader<T>s. there must be a better way!
    loaders: Vec<Resources>,
    extension_to_handler_index: HashMap<String, usize>,
    extension_to_loader_index: HashMap<String, usize>,
    asset_info: RwLock<HashMap<HandleId, AssetInfo>>,
    asset_info_paths: RwLock<HashMap<PathBuf, HandleId>>,
    #[cfg(feature = "filesystem_watcher")]
    filesystem_watcher: Arc<RwLock<Option<FilesystemWatcher>>>,
}

impl Default for AssetServer {
    fn default() -> Self {
        AssetServer {
            #[cfg(feature = "filesystem_watcher")]
            filesystem_watcher: Arc::new(RwLock::new(None)),
            max_loader_threads: 4,
            asset_folders: Default::default(),
            loader_threads: Default::default(),
            asset_handlers: Default::default(),
            loaders: Default::default(),
            extension_to_handler_index: Default::default(),
            extension_to_loader_index: Default::default(),
            asset_info_paths: Default::default(),
            asset_info: Default::default(),
        }
    }
}

impl AssetServer {
    pub fn add_handler<T>(&mut self, asset_handler: T)
    where
        T: AssetLoadRequestHandler,
    {
        let mut asset_handlers = self.asset_handlers.write().unwrap();
        let handler_index = asset_handlers.len();
        for extension in asset_handler.extensions().iter() {
            self.extension_to_handler_index
                .insert(extension.to_string(), handler_index);
        }

        asset_handlers.push(Box::new(asset_handler));
    }

    pub fn add_loader<TLoader, TAsset>(&mut self, loader: TLoader)
    where
        TLoader: AssetLoader<TAsset>,
        TAsset: 'static,
    {
        let loader_index = self.loaders.len();
        for extension in loader.extensions().iter() {
            self.extension_to_loader_index
                .insert(extension.to_string(), loader_index);
        }

        let mut resources = Resources::default();
        resources.insert::<Box<dyn AssetLoader<TAsset>>>(Box::new(loader));
        self.loaders.push(resources);
    }

    pub fn load_asset_folder<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<HandleId>, AssetServerError> {
        let root_path = self.get_root_path()?;
        let asset_folder = root_path.join(path);
        let handle_ids = self.load_assets_in_folder_recursive(&asset_folder)?;
        self.asset_folders.write().unwrap().push(asset_folder);
        Ok(handle_ids)
    }

    pub fn get_handle<T, P: AsRef<Path>>(&self, path: P) -> Option<Handle<T>> {
        self.asset_info_paths
            .read()
            .unwrap()
            .get(path.as_ref())
            .map(|handle_id| Handle::from(*handle_id))
    }

    #[cfg(feature = "filesystem_watcher")]
    fn watch_path_for_changes<P: AsRef<Path>>(
        filesystem_watcher: &mut Option<FilesystemWatcher>,
        path: P,
    ) -> Result<(), AssetServerError> {
        if let Some(watcher) = filesystem_watcher {
            watcher
                .watch(&path)
                .map_err(|_error| AssetServerError::AssetWatchError {
                    path: path.as_ref().to_owned(),
                })?;
        }

        Ok(())
    }

    #[cfg(feature = "filesystem_watcher")]
    pub fn watch_for_changes(&self) -> Result<(), AssetServerError> {
        let mut filesystem_watcher = self.filesystem_watcher.write().unwrap();

        let _ = filesystem_watcher.get_or_insert_with(FilesystemWatcher::default);
        // watch current files
        let asset_info_paths = self.asset_info_paths.read().unwrap();
        for asset_path in asset_info_paths.keys() {
            Self::watch_path_for_changes(&mut filesystem_watcher, asset_path)?;
        }

        Ok(())
    }

    #[cfg(feature = "filesystem_watcher")]
    pub fn filesystem_watcher_system(asset_server: Res<AssetServer>) {
        use notify::event::{Event, EventKind, ModifyKind};
        let mut changed = HashSet::new();

        while let Some(filesystem_watcher) =
            asset_server.filesystem_watcher.read().unwrap().as_ref()
        {
            let result = match filesystem_watcher.receiver.try_recv() {
                Ok(result) => result,
                Err(TryRecvError::Empty) => {
                    break;
                }
                Err(TryRecvError::Disconnected) => panic!("FilesystemWatcher disconnected"),
            };

            let event = result.unwrap();
            if let Event {
                kind: EventKind::Modify(ModifyKind::Data(_)),
                paths,
                ..
            } = event
            {
                for path in paths.iter() {
                    if !changed.contains(path) {
                        let root_path = asset_server.get_root_path().unwrap();
                        let relative_path = path.strip_prefix(root_path).unwrap();
                        match asset_server.load_untyped(relative_path) {
                            Ok(_) => {}
                            Err(AssetServerError::AssetLoadError(error)) => panic!("{:?}", error),
                            Err(_) => {}
                        }
                    }
                }
                changed.extend(paths);
            }
        }
    }

    fn get_root_path(&self) -> Result<PathBuf, AssetServerError> {
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            Ok(PathBuf::from(manifest_dir))
        } else {
            match env::current_exe() {
                Ok(exe_path) => exe_path
                    .parent()
                    .ok_or(AssetServerError::InvalidRootPath)
                    .map(|exe_parent_path| exe_parent_path.to_owned()),
                Err(err) => Err(AssetServerError::Io(err)),
            }
        }
    }

    // TODO: add type checking here. people shouldn't be able to request a Handle<Texture> for a Mesh asset
    pub fn load<T, P: AsRef<Path>>(&self, path: P) -> Result<Handle<T>, AssetServerError> {
        self.load_untyped(path).map(Handle::from)
    }

    pub fn load_sync<T: Resource, P: AsRef<Path>>(
        &self,
        assets: &mut Assets<T>,
        path: P,
    ) -> Result<Handle<T>, AssetServerError>
    where
        T: 'static,
    {
        let path = path.as_ref();
        if let Some(ref extension) = path.extension() {
            if let Some(index) = self.extension_to_loader_index.get(
                extension
                    .to_str()
                    .expect("extension should be a valid string"),
            ) {
                let handle_id = HandleId::new();
                let resources = &self.loaders[*index];
                let loader = resources.get::<Box<dyn AssetLoader<T>>>().unwrap();
                let asset = loader.load_from_file(path)?;
                let handle = Handle::from(handle_id);
                assets.set(handle, asset);
                Ok(handle)
            } else {
                Err(AssetServerError::MissingAssetHandler)
            }
        } else {
            Err(AssetServerError::MissingAssetHandler)
        }
    }

    pub fn load_untyped<P: AsRef<Path>>(&self, path: P) -> Result<HandleId, AssetServerError> {
        let path = path.as_ref();
        if let Some(ref extension) = path.extension() {
            if let Some(index) = self.extension_to_handler_index.get(
                extension
                    .to_str()
                    .expect("Extension should be a valid string."),
            ) {
                let mut new_version = 0;
                let handle_id = {
                    let mut asset_info = self.asset_info.write().unwrap();
                    let mut asset_info_paths = self.asset_info_paths.write().unwrap();
                    if let Some(asset_info) = asset_info_paths
                        .get(path)
                        .and_then(|handle_id| asset_info.get_mut(&handle_id))
                    {
                        asset_info.load_state =
                            if let LoadState::Loaded(_version) = asset_info.load_state {
                                new_version += 1;
                                LoadState::Loading(new_version)
                            } else {
                                LoadState::Loading(new_version)
                            };
                        asset_info.handle_id
                    } else {
                        let handle_id = HandleId::new();
                        asset_info.insert(
                            handle_id,
                            AssetInfo {
                                handle_id,
                                path: path.to_owned(),
                                load_state: LoadState::Loading(new_version),
                            },
                        );
                        asset_info_paths.insert(path.to_owned(), handle_id);
                        handle_id
                    }
                };

                self.send_request_to_loader_thread(LoadRequest {
                    handle_id,
                    path: path.to_owned(),
                    handler_index: *index,
                    version: new_version,
                });

                // TODO: watching each asset explicitly is a simpler implementation, its possible it would be more efficient to watch
                // folders instead (when possible)
                #[cfg(feature = "filesystem_watcher")]
                Self::watch_path_for_changes(&mut self.filesystem_watcher.write().unwrap(), path)?;
                Ok(handle_id)
            } else {
                Err(AssetServerError::MissingAssetHandler)
            }
        } else {
            Err(AssetServerError::MissingAssetHandler)
        }
    }

    pub fn set_load_state(&self, handle_id: HandleId, load_state: LoadState) {
        if let Some(asset_info) = self.asset_info.write().unwrap().get_mut(&handle_id) {
            if load_state.get_version() >= asset_info.load_state.get_version() {
                asset_info.load_state = load_state;
            }
        }
    }

    pub fn get_load_state_untyped(&self, handle_id: HandleId) -> Option<LoadState> {
        self.asset_info
            .read()
            .unwrap()
            .get(&handle_id)
            .map(|asset_info| asset_info.load_state.clone())
    }

    pub fn get_load_state<T>(&self, handle: Handle<T>) -> Option<LoadState> {
        self.get_load_state_untyped(handle.id)
    }

    pub fn get_group_load_state(&self, handle_ids: &[HandleId]) -> Option<LoadState> {
        let mut load_state = LoadState::Loaded(0);
        for handle_id in handle_ids.iter() {
            match self.get_load_state_untyped(*handle_id) {
                Some(LoadState::Loaded(_)) => continue,
                Some(LoadState::Loading(_)) => {
                    load_state = LoadState::Loading(0);
                }
                Some(LoadState::Failed(_)) => return Some(LoadState::Failed(0)),
                None => return None,
            }
        }

        Some(load_state)
    }

    fn send_request_to_loader_thread(&self, load_request: LoadRequest) {
        // NOTE: This lock makes the call to Arc::strong_count safe. Removing (or reordering) it could result in undefined behavior
        let mut loader_threads = self.loader_threads.write().unwrap();
        if loader_threads.len() < self.max_loader_threads {
            let loader_thread = LoaderThread {
                requests: Arc::new(RwLock::new(vec![load_request])),
            };
            let requests = loader_thread.requests.clone();
            loader_threads.push(loader_thread);
            Self::start_thread(self.asset_handlers.clone(), requests);
        } else {
            let most_free_thread = loader_threads
                .iter()
                .min_by_key(|l| l.requests.read().unwrap().len())
                .unwrap();
            let mut requests = most_free_thread.requests.write().unwrap();
            requests.push(load_request);
            // if most free thread only has one reference, the thread as spun down. if so, we need to spin it back up!
            if Arc::strong_count(&most_free_thread.requests) == 1 {
                Self::start_thread(
                    self.asset_handlers.clone(),
                    most_free_thread.requests.clone(),
                );
            }
        }
    }

    fn start_thread(
        request_handlers: Arc<RwLock<Vec<Box<dyn AssetLoadRequestHandler>>>>,
        requests: Arc<RwLock<Vec<LoadRequest>>>,
    ) {
        thread::spawn(move || {
            loop {
                let request = {
                    let mut current_requests = requests.write().unwrap();
                    if current_requests.len() == 0 {
                        // if there are no requests, spin down the thread
                        break;
                    }

                    current_requests.pop().unwrap()
                };

                let handlers = request_handlers.read().unwrap();
                let request_handler = &handlers[request.handler_index];
                request_handler.handle_request(&request);
            }
        });
    }

    fn load_assets_in_folder_recursive(
        &self,
        path: &Path,
    ) -> Result<Vec<HandleId>, AssetServerError> {
        if !path.is_dir() {
            return Err(AssetServerError::AssetFolderNotADirectory(
                path.to_str().unwrap().to_string(),
            ));
        }

        let root_path = self.get_root_path()?;
        let mut handle_ids = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            if child_path.is_dir() {
                handle_ids.extend(self.load_assets_in_folder_recursive(&child_path)?);
            } else {
                let relative_child_path = child_path.strip_prefix(&root_path).unwrap();
                let handle = match self.load_untyped(
                    relative_child_path
                        .to_str()
                        .expect("Path should be a valid string"),
                ) {
                    Ok(handle) => handle,
                    Err(AssetServerError::MissingAssetHandler) => continue,
                    Err(err) => return Err(err),
                };

                handle_ids.push(handle);
            }
        }

        Ok(handle_ids)
    }
}
