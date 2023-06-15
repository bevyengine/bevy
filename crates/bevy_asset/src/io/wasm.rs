use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWatcher, EmptyPathStream, PathStream,
    Reader, VecReader,
};
use anyhow::Result;
use bevy_log::error;
use bevy_utils::BoxedFuture;
use js_sys::Uint8Array;
use std::path::{Path, PathBuf};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

/// Reader implementation for loading assets via HTTP in WASM.
pub struct HttpWasmAssetReader {
    root_path: PathBuf,
}

impl HttpWasmAssetReader {
    /// Creates a new `WasmAssetReader`. The path provided will be used to build URLs to query for assets.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root_path: path.as_ref().to_owned(),
        }
    }
}

impl AssetReader for HttpWasmAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            let path = self.root_path.join(path);
            let window = web_sys::window().unwrap();
            let resp_value = JsFuture::from(window.fetch_with_str(path.to_str().unwrap()))
                .await
                .unwrap();
            let resp: Response = resp_value.dyn_into().unwrap();
            if resp.status() == 404 {
                return Err(AssetReaderError::NotFound(path));
            }
            let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
            let bytes = Uint8Array::new(&data).to_vec();
            let reader: Box<Reader> = Box::new(VecReader::new(bytes));
            Ok(reader)
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            let meta_path = get_meta_path(path);
            let path = self.root_path.join(meta_path);
            let window = web_sys::window().unwrap();
            let resp_value = JsFuture::from(window.fetch_with_str(path.to_str().unwrap()))
                .await
                .unwrap();
            let resp: Response = resp_value.dyn_into().unwrap();
            if resp.status() == 404 {
                return Err(AssetReaderError::NotFound(path));
            }
            let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
            let bytes = Uint8Array::new(&data).to_vec();
            let reader: Box<Reader> = Box::new(VecReader::new(bytes));
            Ok(reader)
        })
    }

    fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        let stream: Box<PathStream> = Box::new(EmptyPathStream);
        error!("Reading directories is not supported with the HttpWasmAssetReader");
        Box::pin(async move { Ok(stream) })
    }

    fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<bool, AssetReaderError>> {
        error!("Reading directories is not supported with the HttpWasmAssetReader");
        Box::pin(async move { Ok(false) })
    }

    fn watch_for_changes(
        &self,
        _event_sender: crossbeam_channel::Sender<super::AssetSourceEvent>,
    ) -> Option<Box<dyn AssetWatcher>> {
        None
    }
}
