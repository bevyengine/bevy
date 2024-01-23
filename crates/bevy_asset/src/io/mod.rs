#[cfg(all(feature = "file_watcher", target_arch = "wasm32"))]
compile_error!(
    "The \"file_watcher\" feature for hot reloading does not work \
    on WASM.\nDisable \"file_watcher\" \
    when compiling to WASM"
);

#[cfg(target_os = "android")]
pub mod android;
pub mod embedded;
#[cfg(not(target_arch = "wasm32"))]
pub mod file;
pub mod gated;
pub mod memory;
pub mod processor_gated;
#[cfg(target_arch = "wasm32")]
pub mod wasm;

mod source;

pub use futures_lite::{AsyncReadExt, AsyncWriteExt};
pub use source::*;

use bevy_utils::{BoxedFuture, ConditionalSend};
use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::{ready, Future, Stream};
use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::Poll,
};
use thiserror::Error;

/// Errors that occur while loading assets.
#[derive(Error, Debug, Clone)]
pub enum AssetReaderError {
    /// Path not found.
    #[error("Path not found: {0}")]
    NotFound(PathBuf),

    /// Encountered an I/O error while loading an asset.
    #[error("Encountered an I/O error while loading asset: {0}")]
    Io(Arc<std::io::Error>),

    /// The HTTP request completed but returned an unhandled [HTTP response status code](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status).
    /// If the request fails before getting a status code (e.g. request timeout, interrupted connection, etc), expect [`AssetReaderError::Io`].
    #[error("Encountered HTTP status {0:?} when loading asset")]
    HttpError(u16),
}

impl From<std::io::Error> for AssetReaderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(Arc::new(value))
    }
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
    ) -> impl Future<Output = Result<Box<Reader<'a>>, AssetReaderError>> + ConditionalSend;
    /// Returns a future to load the full file data at the provided path.
    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl Future<Output = Result<Box<Reader<'a>>, AssetReaderError>> + ConditionalSend;
    /// Returns an iterator of directory entry names at the provided path.
    fn read_directory(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<Box<PathStream>, AssetReaderError>> + ConditionalSend;
    /// Returns an iterator of directory entry names at the provided path.
    fn is_directory(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<bool, AssetReaderError>> + ConditionalSend;
    /// Reads asset metadata bytes at the given `path` into a [`Vec<u8>`]. This is a convenience
    /// function that wraps [`AssetReader::read_meta`] by default.
    fn read_meta_bytes(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<Vec<u8>, AssetReaderError>> + ConditionalSend {
        async {
            let mut meta_reader = self.read_meta(path).await?;
            let mut meta_bytes = Vec::new();
            meta_reader.read_to_end(&mut meta_bytes).await?;
            Ok(meta_bytes)
        }
    }
}

/// Equivalent to an [`AssetReader`] but using boxed futures, necessary eg. when using a `dyn AssetReader`,
/// as [`AssetReader`] isn't currently object safe.
pub trait ErasedAssetReader: Send + Sync + 'static {
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
    /// Returns true if the provided path points to a directory.
    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>>;
    /// Reads asset metadata bytes at the given `path` into a [`Vec<u8>`]. This is a convenience
    /// function that wraps [`ErasedAssetReader::read_meta`] by default.
    fn read_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Vec<u8>, AssetReaderError>>;
}

impl<T: AssetReader> ErasedAssetReader for T {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(Self::read(self, path))
    }
    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(Self::read_meta(self, path))
    }
    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(Self::read_directory(self, path))
    }
    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        Box::pin(Self::is_directory(self, path))
    }
    fn read_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Vec<u8>, AssetReaderError>> {
        Box::pin(Self::read_meta_bytes(self, path))
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
    fn write(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<Box<Writer>, AssetWriterError>> + ConditionalSend;
    /// Writes the full asset meta bytes at the provided path.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn write_meta(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<Box<Writer>, AssetWriterError>> + ConditionalSend;
    /// Removes the asset stored at the given path.
    fn remove(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend;
    /// Removes the asset meta stored at the given path.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn remove_meta(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend;
    /// Renames the asset at `old_path` to `new_path`
    fn rename(
        &self,
        old_path: &Path,
        new_path: &Path,
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend;
    /// Renames the asset meta for the asset at `old_path` to `new_path`.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn rename_meta(
        &self,
        old_path: &Path,
        new_path: &Path,
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend;
    /// Removes the directory at the given path, including all assets _and_ directories in that directory.
    fn remove_directory(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend;
    /// Removes the directory at the given path, but only if it is completely empty. This will return an error if the
    /// directory is not empty.
    fn remove_empty_directory(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend;
    /// Removes all assets (and directories) in this directory, resulting in an empty directory.
    fn remove_assets_in_directory(
        &self,
        path: &Path,
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend;
    /// Writes the asset `bytes` to the given `path`.
    fn write_bytes(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend {
        async {
            let mut writer = self.write(path).await?;
            writer.write_all(bytes).await?;
            writer.flush().await?;
            Ok(())
        }
    }
    /// Writes the asset meta `bytes` to the given `path`.
    fn write_meta_bytes(
        &self,
        path: &Path,
        bytes: &[u8],
    ) -> impl Future<Output = Result<(), AssetWriterError>> + ConditionalSend {
        async {
            let mut meta_writer = self.write_meta(path).await?;
            meta_writer.write_all(bytes).await?;
            meta_writer.flush().await?;
            Ok(())
        }
    }
}

/// Equivalent to an [`AssetWriter`] but using boxed futures, necessary eg. when using a `dyn AssetWriter`,
/// as [`AssetWriter`] isn't currently object safe.
pub trait ErasedAssetWriter: Send + Sync + 'static {
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
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
    /// Writes the asset meta `bytes` to the given `path`.
    fn write_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>>;
}

impl<T: AssetWriter> ErasedAssetWriter for T {
    fn write<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>> {
        Box::pin(Self::write(self, path))
    }
    fn write_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>> {
        Box::pin(Self::write_meta(self, path))
    }
    fn remove<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::remove(self, path))
    }
    fn remove_meta<'a>(&'a self, path: &'a Path) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::remove_meta(self, path))
    }
    fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::rename(self, old_path, new_path))
    }
    fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::rename_meta(self, old_path, new_path))
    }
    fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::remove_directory(self, path))
    }
    fn remove_empty_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::remove_empty_directory(self, path))
    }
    fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::remove_assets_in_directory(self, path))
    }
    fn write_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> BoxedFuture<Result<(), AssetWriterError>> {
        Box::pin(Self::write_bytes(self, path, bytes))
    }
    fn write_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> BoxedFuture<Result<(), AssetWriterError>> {
        Box::pin(Self::write_meta_bytes(self, path, bytes))
    }
}

/// An "asset source change event" that occurs whenever asset (or asset metadata) is created/added/removed
#[derive(Clone, Debug, PartialEq, Eq)]
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
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_io::Result<usize>> {
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
    let mut extension = path.extension().unwrap_or_default().to_os_string();
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
