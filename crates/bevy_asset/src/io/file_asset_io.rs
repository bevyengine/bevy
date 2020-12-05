use crate::{filesystem_watcher::FilesystemWatcher, AssetIo, AssetIoError, AssetServer};
use anyhow::Result;
use bevy_ecs::{bevy_utils::BoxedFuture, Res};
use bevy_utils::HashSet;
use crossbeam_channel::TryRecvError;
use fs::File;
use io::Read;
use parking_lot::RwLock;
use std::{
    env, fs, io,
    path::{Path, PathBuf},
    sync::Arc,
};

pub struct FileAssetIo {
    root_path: PathBuf,
    #[cfg(feature = "filesystem_watcher")]
    filesystem_watcher: Arc<RwLock<Option<FilesystemWatcher>>>,
}

impl FileAssetIo {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        FileAssetIo {
            filesystem_watcher: Default::default(),
            root_path: Self::get_root_path().join(path.as_ref()),
        }
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

    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError> {
        #[cfg(feature = "filesystem_watcher")]
        {
            let path = self.root_path.join(path);
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

        Ok(())
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.root_path.join(path).is_dir()
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
                for path in paths.iter() {
                    if !changed.contains(path) {
                        let relative_path = path.strip_prefix(&asset_io.root_path).unwrap();
                        let _ = asset_server.load_untracked(relative_path, true);
                    }
                }
                changed.extend(paths);
            }
        }
    }
}
