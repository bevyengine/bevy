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
use std::io;
use thiserror::Error;

/// Errors that occur while loading assets
#[derive(Error, Debug)]
pub enum AssetIoError {
    #[error("Path not found")]
    NotFound(String),
    #[error("Encountered an io error while loading asset.")]
    Io(#[from] io::Error),
    #[error("Failed to watch path")]
    PathWatchError(String),
}

/// Handles load requests from an AssetServer
pub trait AssetIo: Downcast + Send + Sync + 'static {
    fn load_path<'a>(&'a self, path: &'a str) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>>;
    fn read_directory(&self, path: &str) -> Result<Box<dyn Iterator<Item = String>>, AssetIoError>;
    fn is_directory(&self, path: &str) -> bool;
    fn watch_path_for_changes(&self, path: &str) -> Result<(), AssetIoError>;
    fn watch_for_changes(&self) -> Result<(), AssetIoError>;
    fn extension<'a>(&self, path: &'a str) -> Option<&'a str>;
    fn parent<'a>(&self, path: &'a str) -> Option<&'a str>;
    fn sibling(&self, path: &str, sibling: &str) -> Option<String>;
}

impl_downcast!(AssetIo);
