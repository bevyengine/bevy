use super::{ChannelAssetHandler, LoadRequest};
use crate::{AssetLoadError, AssetLoader, AssetResult, Handle};
use anyhow::Result;
use async_trait::async_trait;
use crossbeam_channel::Sender;
use std::{fs::File, io::Read};

/// Handles load requests from an AssetServer

#[async_trait]
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

    fn load_asset(&self, load_request: &LoadRequest) -> Result<TAsset, AssetLoadError> {
        match File::open(&load_request.path) {
            Ok(mut file) => {
                let mut bytes = Vec::new();
                file.read_to_end(&mut bytes)?;
                let asset = self.loader.from_bytes(&load_request.path, bytes)?;
                Ok(asset)
            }
            Err(e) => Err(AssetLoadError::Io(std::io::Error::new(
                e.kind(),
                format!("{}", load_request.path.display()),
            ))),
        }
    }
}

#[async_trait]
impl<TLoader, TAsset> AssetLoadRequestHandler for ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset> + 'static,
    TAsset: Send + 'static,
{
    async fn handle_request(&self, load_request: &LoadRequest) {
        let result = self.load_asset(load_request);
        let asset_result = AssetResult {
            handle: Handle::from(load_request.handle_id),
            result,
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
