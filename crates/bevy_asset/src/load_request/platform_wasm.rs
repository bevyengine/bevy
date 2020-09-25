use super::{ChannelAssetHandler, LoadRequest};
use crate::{AssetLoadError, AssetLoader, AssetResult, Handle};
use anyhow::Result;
use async_trait::async_trait;
use crossbeam_channel::Sender;

use js_sys::Uint8Array;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;
use web_sys::Response;

#[async_trait(?Send)]
pub trait AssetLoadRequestHandler: Send + Sync + 'static {
    async fn handle_request(&self, load_request: &LoadRequest);
    fn extensions(&self) -> &[&str];
}

impl<TLoader, TAsset> ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset>,
{
    pub fn new(loader: TLoader, sender: Sender<AssetResult<TAsset>>) -> Self {
        ChannelAssetHandler { sender, loader }
    }

    async fn load_asset(&self, load_request: &LoadRequest) -> Result<TAsset, AssetLoadError> {
        // TODO - get rid of some unwraps below (do some retrying maybe?)
        let window = web_sys::window().unwrap();
        let resp_value = JsFuture::from(window.fetch_with_str(load_request.path.to_str().unwrap()))
            .await
            .unwrap();
        let resp: Response = resp_value.dyn_into().unwrap();
        let data = JsFuture::from(resp.array_buffer().unwrap()).await.unwrap();
        let bytes = Uint8Array::new(&data).to_vec();
        let asset = self.loader.from_bytes(&load_request.path, bytes).unwrap();
        Ok(asset)
    }
}

#[async_trait(?Send)]
impl<TLoader, TAsset> AssetLoadRequestHandler for ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset> + 'static,
    TAsset: Send + 'static,
{
    async fn handle_request(&self, load_request: &LoadRequest) {
        let asset = self.load_asset(load_request).await;
        let asset_result = AssetResult {
            handle: Handle::from(load_request.handle_id),
            result: asset,
            path: load_request.path.clone(),
            version: load_request.version,
        };
        self.sender
            .send(asset_result)
            .expect("loaded asset should have been sent");
    }

    fn extensions(&self) -> &[&str] {
        self.loader.extensions()
    }
}
