use crate::{AssetIo, AssetIoError, ChangeWatcher, Metadata};
use anyhow::Result;
use bevy_utils::BoxedFuture;
use std::{
    convert::TryFrom,
    ffi::CString,
    path::{Path, PathBuf},
};

/// I/O implementation for Android devices.
///
/// Implementation details:
///
/// - [`load_path`](AssetIo::load_path) uses the [`AssetManager`] to load files.
/// - [`read_directory`](AssetIo::read_directory) always returns an empty iterator.
/// - [`get_metadata`](AssetIo::get_metadata) will probably return an error.
/// - Watching for changes is not supported. The watcher methods will do nothing.
///
/// [AssetManager]: https://developer.android.com/reference/android/content/res/AssetManager
pub struct AndroidAssetIo {
    root_path: PathBuf,
}

impl AndroidAssetIo {
    /// Creates a new [`AndroidAssetIo`] at the given root path
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        AndroidAssetIo {
            root_path: path.as_ref().to_owned(),
        }
    }
}

impl AssetIo for AndroidAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            let asset_manager = bevy_winit::ANDROID_APP
                .get()
                .expect("Bevy must be setup with the #[bevy_main] macro on Android")
                .asset_manager();
            let mut opened_asset = asset_manager
                .open(&CString::new(path.to_str().unwrap()).unwrap())
                .ok_or(AssetIoError::NotFound(path.to_path_buf()))?;
            let bytes = opened_asset.get_buffer()?;
            Ok(bytes.to_vec())
        })
    }

    fn read_directory(
        &self,
        _path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        Ok(Box::new(std::iter::empty::<PathBuf>()))
    }

    fn watch_path_for_changes(
        &self,
        _to_watch: &Path,
        _to_reload: Option<PathBuf>,
    ) -> Result<(), AssetIoError> {
        Ok(())
    }

    fn watch_for_changes(&self, _configuration: &ChangeWatcher) -> Result<(), AssetIoError> {
        bevy_log::warn!("Watching for changes is not supported on Android");
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
