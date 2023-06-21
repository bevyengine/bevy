#[cfg(feature = "filesystem_watcher")]
use crate::{filesystem_watcher::FilesystemWatcher, AssetServer};
use crate::{AssetIo, AssetIoError, ChangeWatcher, Metadata};
use anyhow::Result;
#[cfg(feature = "filesystem_watcher")]
use bevy_ecs::system::{Local, Res};
use bevy_utils::BoxedFuture;
#[cfg(feature = "filesystem_watcher")]
use bevy_utils::{default, HashMap, Instant};
#[cfg(feature = "filesystem_watcher")]
use crossbeam_channel::TryRecvError;
use fs::File;
#[cfg(feature = "filesystem_watcher")]
use parking_lot::RwLock;
#[cfg(feature = "filesystem_watcher")]
use std::sync::Arc;
use std::{
    convert::TryFrom,
    env, fs,
    io::Read,
    path::{Path, PathBuf},
};

/// I/O implementation for the local filesystem.
///
/// This asset I/O is fully featured but it's not available on `android` and `wasm` targets.
pub struct FileAssetIo {
    root_path: PathBuf,
    #[cfg(feature = "filesystem_watcher")]
    filesystem_watcher: Arc<RwLock<Option<FilesystemWatcher>>>,
}

impl FileAssetIo {
    /// Creates a new `FileAssetIo` at a path relative to the executable's directory, optionally
    /// watching for changes.
    ///
    /// See `get_base_path` below.
    pub fn new<P: AsRef<Path>>(path: P, watch_for_changes: &Option<ChangeWatcher>) -> Self {
        let file_asset_io = FileAssetIo {
            #[cfg(feature = "filesystem_watcher")]
            filesystem_watcher: default(),
            root_path: Self::get_base_path().join(path.as_ref()),
        };
        if let Some(configuration) = watch_for_changes {
            #[cfg(any(
                not(feature = "filesystem_watcher"),
                target_arch = "wasm32",
                target_os = "android"
            ))]
            panic!(
                "Watch for changes requires the filesystem_watcher feature and cannot be used on \
                wasm32 / android targets"
            );
            #[cfg(feature = "filesystem_watcher")]
            file_asset_io.watch_for_changes(configuration).unwrap();
        }
        file_asset_io
    }

    /// Returns the base path of the assets directory, which is normally the executable's parent
    /// directory.
    ///
    /// If a `BEVY_ASSET_ROOT` environment variable is set, then its value will be used.
    ///
    /// Else if the `CARGO_MANIFEST_DIR` environment variable is set, then its value will be used
    /// instead. It's set by cargo when running with `cargo run`.
    pub fn get_base_path() -> PathBuf {
        if let Ok(env_bevy_asset_root) = env::var("BEVY_ASSET_ROOT") {
            PathBuf::from(env_bevy_asset_root)
        } else if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
            PathBuf::from(manifest_dir)
        } else {
            env::current_exe()
                .map(|path| {
                    path.parent()
                        .map(|exe_parent_path| exe_parent_path.to_owned())
                        .unwrap()
                })
                .unwrap()
        }
    }

    /// Returns the root directory where assets are loaded from.
    ///
    /// See [`get_base_path`](FileAssetIo::get_base_path).
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }
}

impl AssetIo for FileAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            let full_path = self.root_path.join(path);
            match File::open(&full_path) {
                Ok(mut file) => {
                    file.read_to_end(&mut bytes)?;
                }
                Err(e) => {
                    return if e.kind() == std::io::ErrorKind::NotFound {
                        Err(AssetIoError::NotFound(full_path))
                    } else {
                        Err(e.into())
                    }
                }
            }
            Ok(bytes)
        })
    }

    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        let root_path = self.root_path.to_owned();
        Ok(Box::new(fs::read_dir(root_path.join(path))?.map(
            move |entry| {
                let path = entry.unwrap().path();
                path.strip_prefix(&root_path).unwrap().to_owned()
            },
        )))
    }

    fn watch_path_for_changes(
        &self,
        to_watch: &Path,
        to_reload: Option<PathBuf>,
    ) -> Result<(), AssetIoError> {
        #![allow(unused_variables)]
        #[cfg(feature = "filesystem_watcher")]
        {
            let to_reload = to_reload.unwrap_or_else(|| to_watch.to_owned());
            let to_watch = self.root_path.join(to_watch);
            let mut watcher = self.filesystem_watcher.write();
            if let Some(ref mut watcher) = *watcher {
                watcher
                    .watch(&to_watch, to_reload)
                    .map_err(|_error| AssetIoError::PathWatchError(to_watch))?;
            }
        }

        Ok(())
    }

    fn watch_for_changes(&self, configuration: &ChangeWatcher) -> Result<(), AssetIoError> {
        #[cfg(feature = "filesystem_watcher")]
        {
            *self.filesystem_watcher.write() = Some(FilesystemWatcher::new(configuration));
        }
        #[cfg(not(feature = "filesystem_watcher"))]
        bevy_log::warn!("Watching for changes is not supported when the `filesystem_watcher` feature is disabled");

        Ok(())
    }

    fn get_metadata(&self, path: &Path) -> Result<Metadata, AssetIoError> {
        let full_path = self.root_path.join(path);
        full_path
            .metadata()
            .and_then(Metadata::try_from)
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    AssetIoError::NotFound(full_path)
                } else {
                    e.into()
                }
            })
    }
}

/// Watches for file changes in the local file system.
#[cfg(all(
    feature = "filesystem_watcher",
    all(not(target_arch = "wasm32"), not(target_os = "android"))
))]
pub fn filesystem_watcher_system(
    asset_server: Res<AssetServer>,
    mut changed: Local<HashMap<PathBuf, Instant>>,
) {
    let asset_io =
        if let Some(asset_io) = asset_server.server.asset_io.downcast_ref::<FileAssetIo>() {
            asset_io
        } else {
            return;
        };
    let watcher = asset_io.filesystem_watcher.read();

    if let Some(ref watcher) = *watcher {
        loop {
            let event = match watcher.receiver.try_recv() {
                Ok(result) => result.unwrap(),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("FilesystemWatcher disconnected."),
            };

            if let notify::event::Event {
                kind: notify::event::EventKind::Modify(_),
                paths,
                ..
            } = event
            {
                for path in &paths {
                    let Some(set) = watcher.path_map.get(path) else {continue};
                    for to_reload in set {
                        // When an asset is modified, note down the timestamp (overriding any previous modification events)
                        changed.insert(to_reload.to_owned(), Instant::now());
                    }
                }
            }
        }

        // Reload all assets whose last modification was at least 50ms ago.
        //
        // When changing and then saving a shader, several modification events are sent in short succession.
        // Unless we wait until we are sure the shader is finished being modified (and that there will be no more events coming),
        // we will sometimes get a crash when trying to reload a partially-modified shader.
        for (to_reload, _) in
            changed.extract_if(|_, last_modified| last_modified.elapsed() >= watcher.delay)
        {
            let _ = asset_server.load_untracked(to_reload.as_path().into(), true);
        }
    }
}
