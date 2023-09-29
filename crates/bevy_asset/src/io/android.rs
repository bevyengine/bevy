use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWatcher, PathStream,
    Reader, VecReader,
};
use anyhow::Result;
use bevy_log::error;
use bevy_utils::BoxedFuture;
use futures_lite::stream;
use std::{
    ffi::{CString, OsString},
    os::unix::ffi::OsStringExt as _,
    path::Path
};

/// [`AssetReader`] implementation for Android devices, built on top of Android's [`AssetManager`].
///
/// Implementation details:
///
/// - [`read`](AssetIo::read) and [`read_directory`](AssetIo::read_directory) use the [`AssetManager`] to load files and directories.
/// - [`is_directory`](AssetIo::is_directory) always returns false.
/// - Watching for changes is not supported. The watcher method will do nothing.
///
/// [AssetManager]: https://developer.android.com/reference/android/content/res/AssetManager
pub struct AndroidAssetReader;

impl AssetReader for AndroidAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            let asset_manager = bevy_winit::ANDROID_APP
                .get()
                .expect("Bevy must be setup with the #[bevy_main] macro on Android")
                .asset_manager();
            let mut opened_asset = asset_manager
                .open(&CString::new(path.to_str().unwrap()).unwrap())
                .ok_or(AssetReaderError::NotFound(path.to_path_buf()))?;
            let bytes = opened_asset.get_buffer()?;
            let reader: Box<Reader> = Box::new(VecReader::new(bytes.to_vec()));
            Ok(reader)
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            let meta_path = get_meta_path(path);
            let asset_manager = bevy_winit::ANDROID_APP
                .get()
                .expect("Bevy must be setup with the #[bevy_main] macro on Android")
                .asset_manager();
            let mut opened_asset = asset_manager
                .open(&CString::new(meta_path.to_str().unwrap()).unwrap())
                .ok_or(AssetReaderError::NotFound(meta_path))?;
            let bytes = opened_asset.get_buffer()?;
            let reader: Box<Reader> = Box::new(VecReader::new(bytes.to_vec()));
            Ok(reader)
        })
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move {
            let asset_manager = bevy_winit::ANDROID_APP
                .get()
                .expect("Bevy must be setup with the #[bevy_main] macro on Android")
                .asset_manager();

            let opened_dir = asset_manager
                .open_dir(&CString::new(path.to_str().unwrap()).unwrap())
                .ok_or(AssetReaderError::NotFound(path.to_path_buf()))?;

            // collecting it since AssetDir can't be sent between threads
            let paths = opened_dir.map(|path| {
                // convert from CString to PathBuf, only works on unix which android is.
                let bytes = path.to_bytes().to_vec();
                OsString::from_vec(bytes).into()
            }).collect::<Vec<_>>();
            let stream: Box<PathStream> = Box::new(stream::iter(paths));
            Ok(stream)
        })
    }

    fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<bool, AssetReaderError>> {
        error!("Reading directories is not supported with the AndroidAssetReader");
        Box::pin(async move { Ok(false) })
    }

    fn watch_for_changes(
        &self,
        _event_sender: crossbeam_channel::Sender<super::AssetSourceEvent>,
    ) -> Option<Box<dyn AssetWatcher>> {
        None
    }
}
