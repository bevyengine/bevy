use crate::io::{
    get_meta_path, AssetReader, AssetReaderError, EmptyPathStream, PathStream, Reader, VecReader,
};
use bevy_utils::tracing::error;
use js_sys::{Uint8Array, JSON};
use std::path::{Path, PathBuf};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

/// Represents the global object in the JavaScript context
#[wasm_bindgen]
extern "C" {
    /// The [Global](https://developer.mozilla.org/en-US/docs/Glossary/Global_object) object.
    type Global;

    /// The [window](https://developer.mozilla.org/en-US/docs/Web/API/Window) global object.
    #[wasm_bindgen(method, getter, js_name = Window)]
    fn window(this: &Global) -> JsValue;

    /// The [WorkerGlobalScope](https://developer.mozilla.org/en-US/docs/Web/API/WorkerGlobalScope) global object.
    #[wasm_bindgen(method, getter, js_name = WorkerGlobalScope)]
    fn worker(this: &Global) -> JsValue;
}

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

fn js_value_to_err(context: &str) -> impl FnOnce(JsValue) -> std::io::Error + '_ {
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
        // The JS global scope includes a self-reference via a specialising name, which can be used to determine the type of global context available.
        let global: Global = js_sys::global().unchecked_into();
        let promise = if !global.window().is_undefined() {
            let window: web_sys::Window = global.unchecked_into();
            window.fetch_with_str(path.to_str().unwrap())
        } else if !global.worker().is_undefined() {
            let worker: web_sys::WorkerGlobalScope = global.unchecked_into();
            worker.fetch_with_str(path.to_str().unwrap())
        } else {
            let error = std::io::Error::new(
                std::io::ErrorKind::Other,
                "Unsupported JavaScript global context",
            );
            return Err(AssetReaderError::Io(error.into()));
        };
        let resp_value = JsFuture::from(promise)
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
            status => Err(AssetReaderError::HttpError(status)),
        }
    }
}

impl AssetReader for HttpWasmAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let path = self.root_path.join(path);
        self.fetch_bytes(path).await
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let meta_path = get_meta_path(&self.root_path.join(path));
        self.fetch_bytes(meta_path).await
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        let stream: Box<PathStream> = Box::new(EmptyPathStream);
        error!("Reading directories is not supported with the HttpWasmAssetReader");
        Ok(stream)
    }

    async fn is_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> std::result::Result<bool, AssetReaderError> {
        error!("Reading directories is not supported with the HttpWasmAssetReader");
        Ok(false)
    }
}
