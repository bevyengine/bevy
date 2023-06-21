use crate::{AssetIo, AssetIoError, ChangeWatcher, Metadata};
use anyhow::Result;
use bevy_utils::BoxedFuture;
use js_sys::Uint8Array;
use std::{
    convert::TryFrom,
    path::{Path, PathBuf},
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

/// I/O implementation for web builds.
///
/// Implementation details:
///
/// - `load_path` makes [fetch()] requests.
/// - `read_directory` always returns an empty iterator.
/// - `get_metadata` will always return an error.
/// - Watching for changes is not supported. The watcher methods will do nothing.
///
/// [fetch()]: https://developer.mozilla.org/en-US/docs/Web/API/fetch
pub struct WasmAssetIo {
    root_path: PathBuf,
}

impl WasmAssetIo {
    /// Creates a new `WasmAssetIo`. The path provided will be used to build URLs to query for assets.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        WasmAssetIo {
            root_path: path.as_ref().to_owned(),
        }
    }
}

impl AssetIo for WasmAssetIo {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>> {
        Box::pin(async move {
            let path = self.root_path.join(path);
            let window = web_sys::window().unwrap();
            let resp_value = JsFuture::from(window.fetch_with_str(path.to_str().unwrap()))
                .await
                .unwrap();
            let resp: Response = resp_value.dyn_into().unwrap();
            let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
            let bytes = Uint8Array::new(&data).to_vec();
            Ok(bytes)
        })
    }

    fn read_directory(
        &self,
        _path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError> {
        bevy_log::warn!("Loading folders is not supported in WASM");
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
        bevy_log::warn!("Watching for changes is not supported in WASM");
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
