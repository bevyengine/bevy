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
    /// Writes the full asset bytes at the provided path.
    fn write<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>>;
    /// Writes the full asset meta bytes at the provided path.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn write_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>>;
    /// Removes the asset stored at the given path.
    fn remove<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Removes the asset meta stored at the given path.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn remove_meta<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Renames the asset at `old_path` to `new_path`
    fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Renames the asset meta for the asset at `old_path` to `new_path`.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Removes the directory at the given path, including all assets _and_ directories in that directory.
    fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Removes the directory at the given path, but only if it is completely empty. This will return an error if the
    /// directory is not empty.
    fn remove_empty_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Removes all assets (and directories) in this directory, resulting in an empty directory.
    fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Writes the asset `bytes` to the given `path`.
    fn write_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(async move {
            let mut writer = self.write(path).await?;
            writer.write_all(bytes).await?;
            writer.flush().await?;
            Ok(())
        })
    }
    /// Writes the asset meta `bytes` to the given `path`.
    fn write_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(async move {
            let mut meta_writer = self.write_meta(path).await?;
            meta_writer.write_all(bytes).await?;
            meta_writer.flush().await?;
            Ok(())
        })
    }
}

/// An "asset source change event" that occurs whenever asset (or asset metadata) is created/added/removed
#[derive(Clone, Debug)]
pub enum AssetSourceEvent {
    /// An asset at this path was added.
    AddedAsset(PathBuf),
    /// An asset at this path was modified.
    ModifiedAsset(PathBuf),
    /// An asset at this path was removed.
    RemovedAsset(PathBuf),
    /// An asset at this path was renamed.
    RenamedAsset { old: PathBuf, new: PathBuf },
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
    /// Something of unknown type was removed. It is the job of the event handler to determine the type.
    /// This exists because notify-rs produces "untyped" rename events without destination paths for unwatched folders, so we can't determine the type of
    /// the rename.
    RemovedUnknown {
        /// The path of the removed asset or folder (undetermined). This could be an asset path or a folder. This will not be a "meta file" path.
        path: PathBuf,
        /// This field is only relevant if `path` is determined to be an asset path (and therefore not a folder). If this field is `true`,
        /// then this event corresponds to a meta removal (not an asset removal) . If `false`, then this event corresponds to an asset removal
        /// (not a meta removal).
        is_meta: bool,
    },
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
