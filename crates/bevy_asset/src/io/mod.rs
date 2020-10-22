#[cfg(not(target_arch = "wasm32"))]
mod file_asset_io;
#[cfg(target_arch = "wasm32")]
mod wasm_asset_io;

#[cfg(not(target_arch = "wasm32"))]
pub use file_asset_io::*;
#[cfg(target_arch = "wasm32")]
pub use wasm_asset_io::*;

use anyhow::Result;
use bevy_ecs::bevy_utils::BoxedFuture;
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
pub trait AssetIo: Downcast + Send + Sync + 'static {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>>;
    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError>;
    fn is_directory(&self, path: &Path) -> bool;
    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError>;
    fn watch_for_changes(&self) -> Result<(), AssetIoError>;
}

impl_downcast!(AssetIo);
