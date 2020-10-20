#[cfg(not(target_arch = "wasm32"))]
mod file_asset_io;
#[cfg(target_arch = "wasm32")]
mod wasm_asset_io;

#[cfg(not(target_arch = "wasm32"))]
pub use file_asset_io::*;
#[cfg(target_arch = "wasm32")]
pub use wasm_asset_io::*;

use anyhow::Result;
use async_trait::async_trait;
use downcast_rs::{impl_downcast, Downcast};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Errors that occur while loading assets
#[derive(Error, Debug)]
pub enum AssetIoError {
    #[error("Path not found")]
    NotFound(PathBuf),
    #[error("Encountered an io error while loading asset.")]
    Io(#[from] io::Error),
    #[error("Failed to watch path")]
    PathWatchError(PathBuf),
}

/// Handles load requests from an AssetServer
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait AssetIo: Downcast + Send + Sync + 'static {
    async fn load_path(&self, path: &Path) -> Result<Vec<u8>, AssetIoError>;
    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError>;
    fn is_directory(&self, path: &Path) -> bool;
    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError>;
    fn watch_for_changes(&self) -> Result<(), AssetIoError>;
}

impl_downcast!(AssetIo);
