use crate::{
    filesystem_watcher::FilesystemWatcher, AssetLoadError, AssetLoadRequestHandler, AssetLoader,
    Assets, Handle, HandleId, LoadRequest,
};
use anyhow::Result;
use crossbeam_channel::TryRecvError;
use legion::prelude::{Res, Resources};
use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    thread,
};
use thiserror::Error;

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

pub struct AssetServer {
    asset_folders: Vec<PathBuf>,
    loader_threads: RwLock<Vec<LoaderThread>>,
    max_loader_threads: usize,
    asset_handlers: Arc<RwLock<Vec<Box<dyn AssetLoadRequestHandler>>>>,
    // TODO: this is a hack to enable retrieving generic AssetLoader<T>s. there must be a better way!
    loaders: Vec<Resources>,
    extension_to_handler_index: HashMap<String, usize>,
    extension_to_loader_index: HashMap<String, usize>,
    path_to_handle: RwLock<HashMap<PathBuf, HandleId>>,
    #[cfg(feature = "filesystem_watcher")]
    filesystem_watcher: Arc<RwLock<Option<FilesystemWatcher>>>,
}

impl Default for AssetServer {
    fn default() -> Self {
        AssetServer {
            max_loader_threads: 4,
            asset_folders: Vec::new(),
            loader_threads: RwLock::new(Vec::new()),
            asset_handlers: Arc::new(RwLock::new(Vec::new())),
            loaders: Vec::new(),
            extension_to_handler_index: HashMap::new(),
            extension_to_loader_index: HashMap::new(),
            path_to_handle: RwLock::new(HashMap::default()),
            #[cfg(feature = "filesystem_watcher")]
            filesystem_watcher: Arc::new(RwLock::new(None)),
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

    pub fn load_asset_folder<P: AsRef<Path>>(&mut self, path: P) -> Result<(), AssetServerError> {
        let root_path = self.get_root_path()?;
        let asset_folder = root_path.join(path);
        self.load_assets_in_folder_recursive(&asset_folder)?;
        self.asset_folders.push(asset_folder);
        Ok(())
    }

    pub fn get_handle<T, P: AsRef<Path>>(&self, path: P) -> Option<Handle<T>> {
        self.path_to_handle
            .read()
            .unwrap()
            .get(path.as_ref())
            .map(|h| Handle::from(*h))
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

        let _ = filesystem_watcher.get_or_insert_with(|| FilesystemWatcher::default());
        // watch current files
        let path_to_handle = self.path_to_handle.read().unwrap();
        for asset_path in path_to_handle.keys() {
            Self::watch_path_for_changes(&mut filesystem_watcher, asset_path)?;
        }

        Ok(())
    }

    #[cfg(feature = "filesystem_watcher")]
    pub fn filesystem_watcher_system(asset_server: Res<AssetServer>) {
        use notify::event::{Event, EventKind, ModifyKind};
        let mut changed = HashSet::new();
        loop {
            let result = if let Some(filesystem_watcher) =
                asset_server.filesystem_watcher.read().unwrap().as_ref()
            {
                match filesystem_watcher.receiver.try_recv() {
                    Ok(result) => result,
                    Err(TryRecvError::Empty) => {
                        break;
                    }
                    Err(TryRecvError::Disconnected) => panic!("FilesystemWatcher disconnected"),
                }
            } else {
                break;
            };

            let event = result.unwrap();
            match event {
                Event {
                    kind: EventKind::Modify(ModifyKind::Data(_)),
                    paths,
                    ..
                } => {
                    for path in paths.iter() {
                        if !changed.contains(path) {
                            let root_path = asset_server.get_root_path().unwrap();
                            let relative_path = path.strip_prefix(root_path).unwrap();
                            match asset_server.load_untyped(relative_path) {
                                Ok(_) => {}
                                Err(AssetServerError::AssetLoadError(error)) => {
                                    panic!("{:?}", error)
                                }
                                Err(_) => {}
                            }
                        }
                    }
                    changed.extend(paths);
                }
                _ => {}
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
        self.load_untyped(path)
            .map(|handle_id| Handle::from(handle_id))
    }

    pub fn load_sync<T, P: AsRef<Path>>(
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
                assets.set_path(handle, path);
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
                let handle_id = {
                    let mut path_to_handle = self.path_to_handle.write().unwrap();
                    if let Some(handle_id) = path_to_handle.get(path) {
                        *handle_id
                    } else {
                        let handle_id = HandleId::new();
                        path_to_handle.insert(path.to_owned(), handle_id.clone());
                        handle_id
                    }
                };

                self.send_request_to_loader_thread(LoadRequest {
                    handle_id,
                    path: path.to_owned(),
                    handler_index: *index,
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

    fn send_request_to_loader_thread(&self, load_request: LoadRequest) {
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

    fn load_assets_in_folder_recursive(&self, path: &Path) -> Result<(), AssetServerError> {
        if !path.is_dir() {
            return Err(AssetServerError::AssetFolderNotADirectory(
                path.to_str().unwrap().to_string(),
            ));
        }

        let root_path = self.get_root_path()?;
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            if !child_path.is_dir() {
                let relative_child_path = child_path.strip_prefix(&root_path).unwrap();
                let _ = self.load_untyped(
                    relative_child_path
                        .to_str()
                        .expect("Path should be a valid string"),
                );
            } else {
                self.load_assets_in_folder_recursive(&child_path)?;
            }
        }

        Ok(())
    }
}
