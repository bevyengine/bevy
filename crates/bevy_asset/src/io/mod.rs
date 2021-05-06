#[cfg(target_os = "android")]
mod android_asset_io;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
mod file_asset_io;
#[cfg(target_arch = "wasm32")]
mod wasm_asset_io;

mod metadata;

#[cfg(target_os = "android")]
pub use android_asset_io::*;
#[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
pub use file_asset_io::*;
#[cfg(target_arch = "wasm32")]
pub use wasm_asset_io::*;

pub use metadata::*;

use anyhow::Result;
use bevy_utils::BoxedFuture;
use downcast_rs::{impl_downcast, Downcast};
use std::{
    io,
    path::{Path, PathBuf},
};
use thiserror::Error;

/// Errors that occur while loading assets
#[derive(Error, Debug)]
pub enum AssetIoError {
    #[error("path not found: {0}")]
    NotFound(PathBuf),
    #[error("encountered an io error while loading asset: {0}")]
    Io(#[from] io::Error),
    #[error("failed to watch path: {0}")]
    PathWatchError(PathBuf),
}

/// Handles load requests from an `AssetServer`
pub trait AssetIo: Downcast + Send + Sync + 'static {
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>>;
    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError>;
    fn get_metadata(&self, path: &Path) -> Result<Metadata, AssetIoError>;
    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError>;
    fn watch_for_changes(&self) -> Result<(), AssetIoError>;

    fn is_directory(&self, path: &Path) -> bool {
        self.get_metadata(path)
            .map(|metadata| metadata.is_directory())
            .unwrap_or(false)
    }

    fn is_file(&self, path: &Path) -> bool {
        self.get_metadata(path)
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
    }
}

impl_downcast!(AssetIo);
