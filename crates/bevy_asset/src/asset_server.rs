use crate::{
    filesystem_watcher::FilesystemWatcher, AssetLoadError, AssetLoader, Assets, ChannelAssetLoader,
    Handle, HandleId, HandleUntyped, LoadData, UntypedLoader,
};
use anyhow::Result;
use bevy_ecs::{Res, Resource};
use crossbeam_channel::TryRecvError;
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::{
    any::TypeId,
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
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
    #[error("cannot load asset as specified type")]
    IncorrectType,
}

/// Info about a specific asset, such as its path and its current load state
#[derive(Clone, Debug)]
struct AssetInfo {
    handle_id: HandleId,
    path: PathBuf,
    load_state: LoadStatus,
}

/// The load state of an asset
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoadStatus {
    Loading(AssetVersion),
    Loaded(AssetVersion),
    Failed(AssetVersion),
}

impl LoadStatus {
    pub fn get_version(&self) -> AssetVersion {
        match *self {
            LoadStatus::Loaded(version) => version,
            LoadStatus::Loading(version) => version,
            LoadStatus::Failed(version) => version,
        }
    }
}

/// Asynchronously loads assets from the filesystem on background threads!
pub struct AssetServer {
    threadpool: ThreadPool,
    asset_folders: RwLock<Vec<PathBuf>>,
    loaders: Vec<(TypeId, Arc<dyn UntypedLoader>)>,
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

            threadpool: ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .expect("unable to create asset server threadpool"),
            asset_folders: Default::default(),
            loaders: Default::default(),
            extension_to_loader_index: Default::default(),
            asset_info_paths: Default::default(),
            asset_info: Default::default(),
        }
    }
}

impl AssetServer {
    /// Asynchronously load assets of type `T` from the
    /// specified folder.
    /// # Note:
    /// Only loads assets of type `T`.
    /// All other items are ignored.
    pub fn load_folder<T, P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<Handle<T>>, AssetServerError> {
        let path = path.as_ref();
        // Could be more efficient
        let root_path = self.get_root_path()?;
        let asset_folder = root_path.join(path);
        let handles = self
            .load_assets_in_folder_recursive(&asset_folder, Some(TypeId::of::<T>()))?
            .into_iter()
            .map(|h| h.id.into())
            .collect();
        self.asset_folders.write().unwrap().push(asset_folder);
        Ok(handles)
    }

    /// Asynchronously load all assets in the specified folder.
    /// The returned handles can be cast into `Handle<T>`
    /// later on.
    pub fn load_folder_all<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> Result<Vec<HandleUntyped>, AssetServerError> {
        let root_path = self.get_root_path()?;
        let asset_folder = root_path.join(path);
        let handles = self.load_assets_in_folder_recursive(&asset_folder, None)?;
        self.asset_folders.write().unwrap().push(asset_folder);
        Ok(handles)
    }

    /// Asynchronously load an asset of type `T` from the specified path.
    /// If the the asset cannot be loaded as a `T`, then `AssetServerError::IncorrectType`
    /// will be returned.
    pub fn load<T, P: AsRef<Path>>(&self, path: P) -> Result<Handle<T>, AssetServerError> {
        let path = path.as_ref();
        Ok(self
            .load_any_internal(path, Some(TypeId::of::<T>()))?
            .id
            .into())
    }

    /// Synchronously load an asset of type `T` from the specified path.
    /// This will block until the asset has been fully loaded.
    pub fn load_sync<T: Resource, P: AsRef<Path>>(
        &self,
        assets: &mut Assets<T>,
        path: P,
    ) -> Result<Handle<T>, AssetServerError>
    where
        T: 'static,
    {
        let path = path.as_ref();
        if let Some(&index) = path.extension().and_then(|ext| {
            self.extension_to_loader_index
                .get(ext.to_str().expect("extension should be a valid string"))
        }) {
            let untyped_loader = Arc::clone(&self.loaders[index].1);
            // Check that the types match.
            let loader: &dyn AssetLoader<Asset = T> = untyped_loader
                .downcast_loader::<T>()
                .expect("tried to request an asset loader of the wrong type");

            let asset = loader.load_from_file(path)?;
            Ok(assets.add(asset))
        } else {
            Err(AssetServerError::MissingAssetHandler)
        }
    }

    /// Load an asset of any type from the specified path.
    /// The returned handle can be cast into `Handle<T>` later
    /// on.
    pub fn load_any<P: AsRef<Path>>(&self, path: P) -> Result<HandleUntyped, AssetServerError> {
        self.load_any_internal(path, None)
    }

    /// Attempt to get the handle to a loaded asset of type `T`.
    /// If the asset has not finished loading, `None` will be returned.
    /// TODO: Check handle type?
    pub fn get_handle<T, P: AsRef<Path>>(&self, path: P) -> Option<Handle<T>> {
        self.asset_info_paths
            .read()
            .unwrap()
            .get(path.as_ref())
            .map(|handle_id| Handle::from(*handle_id))
    }

    /// Start watching for changes to already registered files and directories.
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
    pub(crate) fn filesystem_watcher_system(asset_server: Res<AssetServer>) {
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
                        match asset_server.load_any(relative_path) {
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

    pub(crate) fn set_load_status(&self, handle_id: HandleId, load_state: LoadStatus) {
        if let Some(asset_info) = self.asset_info.write().unwrap().get_mut(&handle_id) {
            if load_state.get_version() >= asset_info.load_state.get_version() {
                asset_info.load_state = load_state;
            }
        }
    }

    /// Get the current load status of an asset indicated by `handle`.
    pub fn get_load_status<H>(&self, handle: H) -> Option<LoadStatus>
    where
        H: Into<HandleId>,
    {
        self.asset_info
            .read()
            .unwrap()
            .get(&handle.into())
            .map(|asset_info| asset_info.load_state.clone())
    }

    /// Check the least-common-denominator load status of a group of asset handles.
    pub fn get_group_load_status<I>(&self, handles: I) -> Option<LoadStatus>
    where
        I: IntoIterator,
        I::Item: Into<HandleId>,
    {
        let mut load_state = LoadStatus::Loaded(0);
        for handle in handles.into_iter() {
            match self.get_load_status(handle) {
                Some(LoadStatus::Loaded(_)) => continue,
                Some(LoadStatus::Loading(_)) => {
                    load_state = LoadStatus::Loading(0);
                }
                Some(LoadStatus::Failed(_)) => return Some(LoadStatus::Failed(0)),
                None => return None,
            }
        }

        Some(load_state)
    }

    fn load_assets_in_folder_recursive(
        &self,
        path: &Path,
        only_ty: Option<TypeId>,
    ) -> Result<Vec<HandleUntyped>, AssetServerError> {
        if !path.is_dir() {
            return Err(AssetServerError::AssetFolderNotADirectory(
                path.to_str().unwrap().to_string(),
            ));
        }

        let root_path = self.get_root_path()?;
        let mut handles = Vec::new();
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let child_path = entry.path();
            if child_path.is_dir() {
                handles.extend(self.load_assets_in_folder_recursive(&child_path, only_ty)?);
            } else {
                let relative_child_path = child_path.strip_prefix(&root_path).unwrap();
                let handle = match self.load_any_internal(relative_child_path, only_ty) {
                    Ok(handle) => handle,
                    Err(AssetServerError::IncorrectType)
                    | Err(AssetServerError::MissingAssetHandler) => continue,
                    Err(err) => return Err(err),
                };

                handles.push(handle);
            }
        }

        Ok(handles)
    }

    fn load_any_internal<P: AsRef<Path>>(
        &self,
        path: P,
        expected_ty: Option<TypeId>,
    ) -> Result<HandleUntyped, AssetServerError> {
        let path = path.as_ref();
        let index = if let Some(&index) = path.extension().and_then(|ext| {
            self.extension_to_loader_index
                .get(ext.to_str().expect("extension should be a valid string"))
        }) {
            index
        } else {
            return Err(AssetServerError::MissingAssetHandler);
        };

        let (type_id, untyped_loader) = &self.loaders[index];

        if let Some(expected_ty) = expected_ty {
            if *type_id != expected_ty {
                return Err(AssetServerError::IncorrectType);
            }
        }

        let untyped_loader = Arc::clone(untyped_loader);

        let mut version = 0;
        let handle_id = {
            let mut asset_info = self.asset_info.write().unwrap();
            let mut asset_info_paths = self.asset_info_paths.write().unwrap();
            if let Some(asset_info) = asset_info_paths
                .get(path)
                .and_then(|handle_id| asset_info.get_mut(&handle_id))
            {
                asset_info.load_state = if let LoadStatus::Loaded(_version) = asset_info.load_state
                {
                    version += 1;
                    LoadStatus::Loading(version)
                } else {
                    LoadStatus::Loading(version)
                };
                asset_info.handle_id
            } else {
                let handle_id = HandleId::new();
                asset_info.insert(
                    handle_id,
                    AssetInfo {
                        handle_id,
                        path: path.to_owned(),
                        load_state: LoadStatus::Loading(version),
                    },
                );
                asset_info_paths.insert(path.to_owned(), handle_id);
                handle_id
            }
        };

        let load_data = LoadData {
            path: path.to_owned(),
            handle_id,
            version,
        };

        self.threadpool
            .spawn_fifo(move || untyped_loader.load_from_file(load_data));

        // TODO: watching each asset explicitly is a simpler implementation, its possible it would be more efficient to watch
        // folders instead (when possible)
        #[cfg(feature = "filesystem_watcher")]
        Self::watch_path_for_changes(&mut self.filesystem_watcher.write().unwrap(), path)?;

        Ok(HandleUntyped {
            id: handle_id,
            type_id: *type_id,
        })
    }

    pub(crate) fn add_cal<A: Send + Sync>(&mut self, cal: ChannelAssetLoader<A>) {
        let loader_index = self.loaders.len();
        for extension in cal.loader.extensions().iter() {
            self.extension_to_loader_index
                .insert(extension.to_string(), loader_index);
        }

        self.loaders.push((TypeId::of::<A>(), Arc::new(cal)));
    }
}
