use crate::{AssetLoadError, AssetLoader, AssetResult, AssetVersion, Handle, HandleId};
use anyhow::Result;
use crossbeam_channel::Sender;
use fs::File;
use io::Read;
use std::{fs, io, path::PathBuf};

/// A request from an [AssetServer](crate::AssetServer) to load an asset.
#[derive(Debug)]
pub struct LoadRequest {
    pub path: PathBuf,
    pub handle_id: HandleId,
    pub handler_index: usize,
    pub version: AssetVersion,
}

/// Handles load requests from an AssetServer
pub trait AssetLoadRequestHandler: Send + Sync + 'static {
    fn handle_request(&self, load_request: &LoadRequest);
    fn extensions(&self) -> &[&str];
}

pub(crate) struct ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset>,
    TAsset: 'static,
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
