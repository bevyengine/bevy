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

/// Errors that occur while loading assets.
#[derive(Error, Debug)]
pub enum AssetIoError {
    /// Path not found.
    #[error("path not found: {0}")]
    NotFound(PathBuf),

    /// Encountered an I/O error while loading an asset.
    #[error("encountered an io error while loading asset: {0}")]
    Io(#[from] io::Error),

    /// Failed to watch path.
    #[error("failed to watch path: {0}")]
    PathWatchError(PathBuf),
}

/// A storage provider for an [`AssetServer`].
///
/// An asset I/O is the backend actually providing data for the asset loaders managed by the asset
/// server. An average user will probably be just fine with the default [`FileAssetIo`], but you
/// can easily use your own custom I/O to, for example, load assets from cloud storage or create a
/// seamless VFS layout using custom containers.
///
/// See the [`custom_asset_io`]  example in the repository for more details.
///
/// [`AssetServer`]: struct.AssetServer.html
/// [`custom_asset_io`]: https://github.com/bevyengine/bevy/tree/latest/examples/asset/custom_asset_io.rs
pub trait AssetIo: Downcast + Send + Sync + 'static {
    /// Returns a future to load the full file data at the provided path.
    fn load_path<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<Vec<u8>, AssetIoError>>;

    /// Returns an iterator of directory entry names at the provided path.
    fn read_directory(
        &self,
        path: &Path,
    ) -> Result<Box<dyn Iterator<Item = PathBuf>>, AssetIoError>;

    /// Returns metadata about the filesystem entry at the provided path.
    fn get_metadata(&self, path: &Path) -> Result<Metadata, AssetIoError>;

    /// Tells the asset I/O to watch for changes recursively at the provided path.
    fn watch_path_for_changes(&self, path: &Path) -> Result<(), AssetIoError>;

    /// Enables change tracking in this asset I/O.
    fn watch_for_changes(&self) -> Result<(), AssetIoError>;

    /// Returns `true` if the path is a directory.
    fn is_dir(&self, path: &Path) -> bool {
        self.get_metadata(path)
            .as_ref()
            .map(Metadata::is_dir)
            .unwrap_or(false)
    }

    /// Returns `true` if the path is a file.
    fn is_file(&self, path: &Path) -> bool {
        self.get_metadata(path)
            .as_ref()
            .map(Metadata::is_file)
            .unwrap_or(false)
    }
}

impl_downcast!(AssetIo);
