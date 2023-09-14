use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, AssetWatcher, EmptyPathStream, PathStream,
    Reader, VecReader,
};
use anyhow::Result;
use bevy_log::error;
use bevy_utils::BoxedFuture;
use js_sys::{Uint8Array, JSON};
use std::path::{Path, PathBuf};
use wasm_bindgen::{JsCast, JsValue};
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

fn js_value_to_err<'a>(context: &'a str) -> impl FnOnce(JsValue) -> std::io::Error + 'a {
    move |value| {
        let message = match JSON::stringify(&value) {
            Ok(js_str) => format!("Failed to {context}: {js_str}"),
            Err(_) => {
                format!("Failed to {context} and also failed to stringify the JSValue of the error")
            }
        };

        std::io::Error::new(std::io::ErrorKind::Other, message)
    }
}

impl HttpWasmAssetReader {
    async fn fetch_bytes<'a>(&self, path: PathBuf) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_str(path.to_str().unwrap()))
            .await
            .map_err(js_value_to_err("fetch path"))?;
        let resp = resp_value
            .dyn_into::<Response>()
            .map_err(js_value_to_err("convert fetch to Response"))?;
        match resp.status() {
            200 => {
                let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
                let bytes = Uint8Array::new(&data).to_vec();
                let reader: Box<Reader> = Box::new(VecReader::new(bytes));
                Ok(reader)
            }
            404 => Err(AssetReaderError::NotFound(path)),
            status => Err(AssetReaderError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Encountered unexpected HTTP status {status}"),
            ))),
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
            self.fetch_bytes(path).await
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            let meta_path = get_meta_path(path);
            Ok(self.fetch_bytes(meta_path).await?)
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
