use crate::{AssetIo, AssetIoError};
use anyhow::Result;
use bevy_ecs::bevy_utils::BoxedFuture;
use js_sys::Uint8Array;
use std::path::{Path, PathBuf};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

pub struct WasmAssetIo {
    root_path: PathBuf,
}

impl WasmAssetIo {
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
        Ok(Box::new(std::iter::empty::<PathBuf>()))
    }

    fn watch_path_for_changes(&self, _path: &Path) -> Result<(), AssetIoError> {
        Ok(())
    }

    fn watch_for_changes(&self) -> Result<(), AssetIoError> {
        Ok(())
    }

    fn is_directory(&self, path: &Path) -> bool {
        self.root_path.join(path).is_dir()
    }
}
