use crate::{AssetLoadError, AssetLoader, AssetPath, AssetResult, Handle, HandleId};
use anyhow::Result;
use crossbeam_channel::Sender;
use fs::File;
use io::Read;
use std::{fs, io, path::Path};

#[derive(Debug)]
pub struct LoadRequest {
    pub path: AssetPath,
    pub handle_id: HandleId,
    pub handler_index: usize,
}

pub trait AssetLoadRequestHandler: Send + Sync + 'static {
    fn handle_request(&self, load_request: &LoadRequest);
    fn extensions(&self) -> &[&str];
}

pub struct ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset>,
{
    sender: Sender<AssetResult<TAsset>>,
    loader: TLoader,
}

impl<TLoader, TAsset> ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset>,
{
    pub fn new(loader: TLoader, sender: Sender<AssetResult<TAsset>>) -> Self {
        ChannelAssetHandler { sender, loader }
    }

    fn load_asset(&self, load_request: &LoadRequest) -> Result<TAsset, AssetLoadError> {
        let mut file = File::open(Path::new(load_request.path.path.as_ref()))?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let asset = self.loader.from_bytes(&load_request.path, bytes)?;
        Ok(asset)
    }
}

impl<TLoader, TAsset> AssetLoadRequestHandler for ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset> + 'static,
    TAsset: Send + 'static,
{
    fn handle_request(&self, load_request: &LoadRequest) {
        let result = self.load_asset(load_request);
        let asset_result = AssetResult {
            handle: Handle::from(load_request.handle_id),
            result,
            path: load_request.path.clone(),
        };
        self.sender
            .send(asset_result)
            .expect("loaded asset should have been sent");
    }
    fn extensions(&self) -> &[&str] {
        self.loader.extensions()
    }
}
