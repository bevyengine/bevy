use crate::{AssetIo, AssetIoError, Metadata};
use anyhow::Result;
use bevy_utils::BoxedFuture;
use std::{
    convert::TryFrom,
    ffi::CString,
    path::{Path, PathBuf},
};

pub struct AndroidAssetIo {
    root_path: PathBuf,
}

impl AndroidAssetIo {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        AndroidAssetIo {
            root_path: path.as_ref().to_owned(),
        }
    }
}

impl AssetIo for AndroidAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            let asset_manager = ndk_glue::native_activity().asset_manager();
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

    fn watch_path_for_changes(&self, _path: &Path) -> Result<(), AssetIoError> {
        Ok(())
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
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
