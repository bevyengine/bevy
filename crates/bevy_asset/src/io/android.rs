use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, EmptyPathStream, PathStream, Reader, VecReader,
};
use bevy_utils::tracing::error;
use std::{ffi::CString, path::Path};

/// [`AssetReader`] implementation for Android devices, built on top of Android's [`AssetManager`].
///
/// Implementation details:
///
/// - [`load_path`](AssetIo::load_path) uses the [`AssetManager`] to load files.
/// - [`read_directory`](AssetIo::read_directory) always returns an empty iterator.
/// - Watching for changes is not supported. The watcher method will do nothing.
///
/// [AssetManager]: https://developer.android.com/reference/android/content/res/AssetManager
pub struct AndroidAssetReader;

impl AssetReader for AndroidAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let asset_manager = bevy_winit::ANDROID_APP
            .get()
            .expect("Bevy must be setup with the #[bevy_main] macro on Android")
            .asset_manager();
        let mut opened_asset = asset_manager
            .open(&CString::new(path.to_str().unwrap()).unwrap())
            .ok_or(AssetReaderError::NotFound(path.to_path_buf()))?;
        let bytes = opened_asset.buffer()?;
        let reader: Box<Reader> = Box::new(VecReader::new(bytes.to_vec()));
        Ok(reader)
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let meta_path = get_meta_path(path);
        let asset_manager = bevy_winit::ANDROID_APP
            .get()
            .expect("Bevy must be setup with the #[bevy_main] macro on Android")
            .asset_manager();
        let mut opened_asset = asset_manager
            .open(&CString::new(meta_path.to_str().unwrap()).unwrap())
            .ok_or(AssetReaderError::NotFound(meta_path))?;
        let bytes = opened_asset.buffer()?;
        let reader: Box<Reader> = Box::new(VecReader::new(bytes.to_vec()));
        Ok(reader)
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let stream: Box<PathStream> = Box::new(EmptyPathStream);
        error!("Reading directories is not supported with the AndroidAssetReader");
        Ok(stream)
    }

    async fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> std::result::Result<bool, AssetReaderError> {
        error!("Reading directories is not supported with the AndroidAssetReader");
        Ok(false)
    }
}
