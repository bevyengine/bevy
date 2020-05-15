use crate::{Assets, Handle, HandleId, LoadRequest, AssetLoadError, AssetLoadRequestHandler, AssetLoader, AssetPath};
use anyhow::Result;
use legion::prelude::Resources;
use std::{
    collections::HashMap,
    env, fs, io,
    path::Path,
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
}

struct LoaderThread {
    // NOTE: these must remain private. the LoaderThread Arc counters are used to determine thread liveness
    // if there is one reference, the loader thread is dead. if there are two references, the loader thread is active
    requests: Arc<RwLock<Vec<LoadRequest>>>,
}

pub struct AssetServer {
    asset_folders: Vec<String>,
    loader_threads: RwLock<Vec<LoaderThread>>,
    max_loader_threads: usize,
    asset_handlers: Arc<RwLock<Vec<Box<dyn AssetLoadRequestHandler>>>>,
    // TODO: this is a hack to enable retrieving generic AssetLoader<T>s. there must be a better way!
    loaders: Vec<Resources>,
    extension_to_handler_index: HashMap<String, usize>,
    extension_to_loader_index: HashMap<String, usize>,
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
        }
    }
}

impl AssetServer {
    pub fn add_handler<T>(&mut self, asset_handler: T)
    where
        T: AssetLoadRequestHandler,
    {
        let mut asset_handlers = self.asset_handlers.write().expect("RwLock poisoned");
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

    pub fn add_asset_folder(&mut self, path: &str) {
        self.asset_folders.push(path.to_string());
    }

    pub fn get_root_path(&self) -> Result<String, AssetServerError> {
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            Ok(manifest_dir)
        } else {
            match env::current_exe() {
                Ok(exe_path) => exe_path
                    .parent()
                    .ok_or(AssetServerError::InvalidRootPath)
                    .and_then(|exe_parent_path| {
                        exe_parent_path
                            .to_str()
                            .map(|path| path.to_string())
                            .ok_or(AssetServerError::InvalidRootPath)
                    }),
                Err(err) => Err(AssetServerError::Io(err)),
            }
        }
    }

    pub fn load_assets(&self) -> Result<(), AssetServerError> {
        let root_path_str = self.get_root_path()?;
        let root_path = Path::new(&root_path_str);
        for folder in self.asset_folders.iter() {
            let asset_folder_path = root_path.join(folder);
            self.load_assets_in_folder_recursive(&asset_folder_path)?;
        }

        Ok(())
    }

    pub fn load<T>(&self, path: &str) -> Result<Handle<T>, AssetServerError> {
        self.load_untyped(path)
            .map(|handle_id| Handle::from(handle_id))
    }

    pub fn load_sync<T>(
        &self,
        assets: &mut Assets<T>,
        path: &str,
    ) -> Result<Handle<T>, AssetServerError>
    where
        T: 'static,
    {
        let asset_path = AssetPath::from(path);
        if let Some(ref extension) = asset_path.extension {
            if let Some(index) = self.extension_to_loader_index.get(extension.as_ref()) {
                let handle_id = HandleId::new();
                let resources = &self.loaders[*index];
                let loader = resources.get::<Box<dyn AssetLoader<T>>>().unwrap();
                let asset = loader.load_from_file(&asset_path)?;
                let handle = Handle::from(handle_id);
                assets.add_with_handle(handle, asset);
                assets.set_path(handle, &asset_path.path);
                Ok(handle)
            } else {
                Err(AssetServerError::MissingAssetHandler)
            }
        } else {
            Err(AssetServerError::MissingAssetHandler)
        }
    }

    pub fn load_untyped(&self, path: &str) -> Result<HandleId, AssetServerError> {
        let asset_path = AssetPath::from(path);
        if let Some(ref extension) = asset_path.extension {
            if let Some(index) = self.extension_to_handler_index.get(extension.as_ref()) {
                let handle_id = HandleId::new();
                self.send_request_to_loader_thread(LoadRequest {
                    handle_id,
                    path: asset_path,
                    handler_index: *index,
                });
                Ok(handle_id)
            } else {
                Err(AssetServerError::MissingAssetHandler)
            }
        } else {
            Err(AssetServerError::MissingAssetHandler)
        }
    }

    fn send_request_to_loader_thread(&self, load_request: LoadRequest) {
        let mut loader_threads = self.loader_threads.write().expect("RwLock poisoned");
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
                .min_by_key(|l| l.requests.read().expect("RwLock poisoned").len())
                .unwrap();
            let mut requests = most_free_thread.requests.write().expect("RwLock poisoned");
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
                    let mut current_requests = requests.write().expect("RwLock poisoned");
                    if current_requests.len() == 0 {
                        // if there are no requests, spin down the thread
                        break;
                    }

                    current_requests.pop().unwrap()
                };

                let handlers = request_handlers.read().expect("RwLock poisoned");
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

        for entry in fs::read_dir(&path)? {
            let entry = entry?;
            let child_path = entry.path();
            if !child_path.is_dir() {
                let _ =
                    self.load_untyped(child_path.to_str().expect("Path should be a valid string"));
            } else {
                self.load_assets_in_folder_recursive(&child_path)?;
            }
        }

        Ok(())
    }
}
