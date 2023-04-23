pub mod file;
pub mod gated;
pub mod memory;
pub mod processor_gated;

mod provider;

use crossbeam_channel::Sender;
pub use futures_lite::{AsyncReadExt, AsyncWriteExt};
pub use provider::*;

use bevy_utils::BoxedFuture;
use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::Stream;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that occur while loading assets.
#[derive(Error, Debug)]
pub enum AssetReaderError {
    /// Path not found.
    #[error("path not found: {0}")]
    NotFound(PathBuf),

    /// Encountered an I/O error while loading an asset.
    #[error("encountered an io error while loading asset: {0}")]
    Io(#[from] std::io::Error),
}

pub type Reader<'a> = dyn AsyncRead + Unpin + Send + Sync + 'a;

pub trait AssetReader: Send + Sync + 'static {
    /// Returns a future to load the full file data at the provided path.
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>>;
    /// Returns a future to load the full file data at the provided path.
    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>>;
    /// Returns an iterator of directory entry names at the provided path.
    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>>;
    /// Returns an iterator of directory entry names at the provided path.
    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>>;

    /// Returns an Asset watcher that will send events on the given channel.
    /// If this reader does not support watching for changes, this will return [`None`].
    fn watch_for_changes(
        &self,
        event_sender: Sender<AssetSourceEvent>,
    ) -> Option<Box<dyn AssetWatcher>>;

    fn read_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Vec<u8>, AssetReaderError>> {
        Box::pin(async move {
            let mut meta_reader = self.read_meta(path).await?;
            let mut meta_bytes = Vec::new();
            meta_reader.read_to_end(&mut meta_bytes).await?;
            Ok(meta_bytes)
        })
    }
}

pub type Writer = dyn AsyncWrite + Unpin + Send + Sync;

pub type PathStream = dyn Stream<Item = PathBuf> + Unpin + Send;

/// Errors that occur while loading assets.
#[derive(Error, Debug)]
pub enum AssetWriterError {
    /// Encountered an I/O error while loading an asset.
    #[error("encountered an io error while loading asset: {0}")]
    Io(#[from] std::io::Error),
}
pub trait AssetWriter: Send + Sync + 'static {
    /// Returns a future to load the full file data at the provided path.
    fn write<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>>;
    fn write_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>>;
    fn remove<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    fn remove_meta<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
}

#[derive(Clone, Debug)]
pub enum AssetSourceEvent {
    Added(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    AddedMeta(PathBuf),
    ModifiedMeta(PathBuf),
    RemovedMeta(PathBuf),
    AddedFolder(PathBuf),
    RemovedFolder(PathBuf),
}

pub trait AssetWatcher: Send + Sync + 'static {}
