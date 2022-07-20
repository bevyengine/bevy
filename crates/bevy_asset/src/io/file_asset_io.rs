#[cfg(feature = "filesystem_watcher")]
use crate::{filesystem_watcher::FilesystemWatcher, AssetServer};
use crate::{AssetIo, AssetIoError, Metadata};
use anyhow::Result;
#[cfg(feature = "filesystem_watcher")]
use bevy_ecs::system::Res;
use bevy_utils::BoxedFuture;
#[cfg(feature = "filesystem_watcher")]
use bevy_utils::HashSet;
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

pub struct FileAssetIo {
    root_path: PathBuf,
    #[cfg(feature = "filesystem_watcher")]
    filesystem_watcher: Arc<RwLock<Option<FilesystemWatcher>>>,
}

impl FileAssetIo {
    pub fn new<P: AsRef<Path>>(path: P, watch_for_changes: bool) -> Self {
        let file_asset_io = FileAssetIo {
            #[cfg(feature = "filesystem_watcher")]
            filesystem_watcher: Default::default(),
            root_path: Self::get_root_path().join(path.as_ref()),
        };
        if watch_for_changes {
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
            file_asset_io.watch_for_changes().unwrap();
        }
        file_asset_io
    }

    pub fn get_root_path() -> PathBuf {
        if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
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

    fn watch_path_for_changes(&self, _path: &Path) -> Result<(), AssetIoError> {
        #[cfg(feature = "filesystem_watcher")]
        {
            let path = self.root_path.join(_path);
            let mut watcher = self.filesystem_watcher.write();
            if let Some(ref mut watcher) = *watcher {
                watcher
                    .watch(&path)
                    .map_err(|_error| AssetIoError::PathWatchError(path))?;
            }
        }

        Ok(())
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        #[cfg(feature = "filesystem_watcher")]
        {
            *self.filesystem_watcher.write() = Some(FilesystemWatcher::default());
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

#[cfg(all(
    feature = "filesystem_watcher",
    all(not(target_arch = "wasm32"), not(target_os = "android"))
))]
pub fn filesystem_watcher_system(asset_server: Res<AssetServer>) {
    let mut changed = HashSet::default();
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
                    if !changed.contains(path) {
                        let relative_path = path.strip_prefix(&asset_io.root_path).unwrap();
                        let _ = asset_server.load_untracked(relative_path.into(), true);
                    }
                }
                changed.extend(paths);
            }
        }
    }
}
