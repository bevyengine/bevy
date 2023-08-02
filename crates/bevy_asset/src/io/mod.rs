#[cfg(target_os = "android")]
pub mod android;
#[cfg(not(target_arch = "wasm32"))]
pub mod file;
pub mod gated;
pub mod memory;
pub mod processor_gated;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

mod provider;

pub use futures_lite::{AsyncReadExt, AsyncWriteExt};
pub use provider::*;

use bevy_utils::BoxedFuture;
use crossbeam_channel::Sender;
use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::{ready, Stream};
use std::{
    path::{Path, PathBuf},
    pin::Pin,
    task::Poll,
};
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

/// Performs read operations on an asset storage. [`AssetReader`] exposes a "virtual filesystem"
/// API, where asset bytes and asset metadata bytes are both stored and accessible for a given
/// `path`.
///
/// Also see [`AssetWriter`].
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

    /// Reads asset metadata bytes at the given `path` into a [`Vec<u8>`]. This is a convenience
    /// function that wraps [`AssetReader::read_meta`] by default.
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

/// Preforms write operations on an asset storage. [`AssetWriter`] exposes a "virtual filesystem"
/// API, where asset bytes and asset metadata bytes are both stored and accessible for a given
/// `path`.
///
/// Also see [`AssetReader`].
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
    fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
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

/// An "asset source change event" that occurs whenever asset (or asset metadata) is created/added/removed
#[derive(Clone, Debug)]
pub enum AssetSourceEvent {
    /// An asset at this path was added.
    Added(PathBuf),
    /// An asset at this path was modified.
    Modified(PathBuf),
    /// An asset at this path was removed.
    Removed(PathBuf),
    /// An asset at this path was renamed.
    Renamed { old: PathBuf, new: PathBuf },
    /// Asset metadata at this path was added.
    AddedMeta(PathBuf),
    /// Asset metadata at this path was modified.
    ModifiedMeta(PathBuf),
    /// Asset metadata at this path was removed.
    RemovedMeta(PathBuf),
    /// Asset metadata at this path was renamed.
    RenamedMeta { old: PathBuf, new: PathBuf },
    /// A folder at the given path was added.
    AddedFolder(PathBuf),
    /// A folder at the given path was removed.
    RemovedFolder(PathBuf),
    /// A folder at the given path was renamed.
    RenamedFolder { old: PathBuf, new: PathBuf },
}

/// A handle to an "asset watcher" process, that will listen for and emit [`AssetSourceEvent`] values for as long as
/// [`AssetWatcher`] has not been dropped.
///
/// See [`AssetReader::watch_for_changes`].
pub trait AssetWatcher: Send + Sync + 'static {}

/// An [`AsyncRead`] implementation capable of reading a [`Vec<u8>`].
pub struct VecReader {
    bytes: Vec<u8>,
    bytes_read: usize,
}

impl VecReader {
    /// Create a new [`VecReader`] for `bytes`.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes_read: 0,
            bytes,
        }
    }
}

impl AsyncRead for VecReader {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<futures_io::Result<usize>> {
        if self.bytes_read >= self.bytes.len() {
            Poll::Ready(Ok(0))
        } else {
            let n = ready!(Pin::new(&mut &self.bytes[self.bytes_read..]).poll_read(cx, buf))?;
            self.bytes_read += n;
            Poll::Ready(Ok(n))
        }
    }
}

/// Appends `.meta` to the given path.
pub(crate) fn get_meta_path(path: &Path) -> PathBuf {
    let mut meta_path = path.to_path_buf();
    let mut extension = path
        .extension()
        .expect("asset paths must have extensions")
        .to_os_string();
    extension.push(".meta");
    meta_path.set_extension(extension);
    meta_path
}

/// A [`PathBuf`] [`Stream`] implementation that immediately returns nothing.
struct EmptyPathStream;

impl Stream for EmptyPathStream {
    type Item = PathBuf;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}
