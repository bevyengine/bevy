use crate::{AssetLoader, AssetResult, AssetVersion, HandleId};
use crossbeam_channel::Sender;
use std::{fmt::Debug, path::PathBuf};

#[cfg(not(target_arch = "wasm32"))]
#[path = "platform_default.rs"]
mod platform_specific;

#[cfg(target_arch = "wasm32")]
#[path = "platform_wasm.rs"]
mod platform_specific;

pub use platform_specific::*;

pub enum DataOrigin {
    Path(PathBuf),
    Read(std::sync::Arc<std::sync::Mutex<Box<(dyn std::io::Read + Send + 'static)>>>),
}

impl Debug for DataOrigin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(path_buf) => f.debug_tuple("DataOrigin::Path").field(path_buf).finish(),
            Self::Read(_) => f.write_fmt(format_args!("DataOrigin::Read")),
        }
    }
}

/// A request from an [AssetServer](crate::AssetServer) to load an asset.
#[derive(Debug)]
pub struct LoadRequest {
    pub path: DataOrigin,
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
