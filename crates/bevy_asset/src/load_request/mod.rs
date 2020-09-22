use crate::{AssetLoader, AssetResult, AssetVersion, HandleId};
use crossbeam_channel::Sender;
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
#[path = "platform_default.rs"]
mod platform_specific;

#[cfg(target_arch = "wasm32")]
#[path = "platform_wasm.rs"]
mod platform_specific;

pub use platform_specific::*;

/// A request from an [AssetServer](crate::AssetServer) to load an asset.
#[derive(Debug)]
pub struct LoadRequest {
    pub path: PathBuf,
    pub handle_id: HandleId,
    pub handler_index: usize,
    pub version: AssetVersion,
}

pub(crate) struct ChannelAssetHandler<TLoader, TAsset>
where
    TLoader: AssetLoader<TAsset>,
    TAsset: 'static,
{
    sender: Sender<AssetResult<TAsset>>,
    loader: TLoader,
}
