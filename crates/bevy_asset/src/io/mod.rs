#[cfg(all(feature = "file_watcher", target_arch = "wasm32"))]
compile_error!(
    "The \"file_watcher\" feature for hot reloading does not work \
    on Wasm.\nDisable \"file_watcher\" \
    when compiling to Wasm"
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

pub use futures_lite::AsyncWriteExt;
pub use source::*;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use core::future::Future;
use core::{
    mem::size_of,
    pin::Pin,
    task::{Context, Poll},
};
use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::{ready, Stream};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that occur while loading assets.
#[derive(Error, Debug, Clone)]
pub enum AssetReaderError {
    /// Path not found.
    #[error("Path not found: {}", _0.display())]
    NotFound(PathBuf),

    /// Encountered an I/O error while loading an asset.
    #[error("Encountered an I/O error while loading asset: {0}")]
    Io(Arc<std::io::Error>),

    /// The HTTP request completed but returned an unhandled [HTTP response status code](https://developer.mozilla.org/en-US/docs/Web/HTTP/Status).
    /// If the request fails before getting a status code (e.g. request timeout, interrupted connection, etc), expect [`AssetReaderError::Io`].
    #[error("Encountered HTTP status {0:?} when loading asset")]
    HttpError(u16),
}

impl PartialEq for AssetReaderError {
    /// Equality comparison for `AssetReaderError::Io` is not full (only through `ErrorKind` of inner error)
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::NotFound(path), Self::NotFound(other_path)) => path == other_path,
            (Self::Io(error), Self::Io(other_error)) => error.kind() == other_error.kind(),
            (Self::HttpError(code), Self::HttpError(other_code)) => code == other_code,
            _ => false,
        }
    }
}

impl Eq for AssetReaderError {}

impl From<std::io::Error> for AssetReaderError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(Arc::new(value))
    }
}

/// The maximum size of a future returned from [`Reader::read_to_end`].
/// This is large enough to fit ten references.
// Ideally this would be even smaller (ReadToEndFuture only needs space for two references based on its definition),
// but compiler optimizations can apparently inflate the stack size of futures due to inlining, which makes
// a higher maximum necessary.
pub const STACK_FUTURE_SIZE: usize = 10 * size_of::<&()>();

pub use stackfuture::StackFuture;

/// Asynchronously advances the cursor position by a specified number of bytes.
///
/// This trait is a simplified version of the [`futures_io::AsyncSeek`] trait, providing
/// support exclusively for the [`futures_io::SeekFrom::Current`] variant. It allows for relative
/// seeking from the current cursor position.
pub trait AsyncSeekForward {
    /// Attempts to asynchronously seek forward by a specified number of bytes from the current cursor position.
    ///
    /// Seeking beyond the end of the stream is allowed and the behavior for this case is defined by the implementation.
    /// The new position, relative to the beginning of the stream, should be returned upon successful completion
    /// of the seek operation.
    ///
    /// If the seek operation completes successfully,
    /// the new position relative to the beginning of the stream should be returned.
    ///
    /// # Implementation
    ///
    /// Implementations of this trait should handle [`Poll::Pending`] correctly, converting
    /// [`std::io::ErrorKind::WouldBlock`] errors into [`Poll::Pending`] to indicate that the operation is not
    /// yet complete and should be retried, and either internally retry or convert
    /// [`std::io::ErrorKind::Interrupted`] into another error kind.
    fn poll_seek_forward(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        offset: u64,
    ) -> Poll<futures_io::Result<u64>>;
}

impl<T: ?Sized + AsyncSeekForward + Unpin> AsyncSeekForward for Box<T> {
    fn poll_seek_forward(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        offset: u64,
    ) -> Poll<futures_io::Result<u64>> {
        Pin::new(&mut **self).poll_seek_forward(cx, offset)
    }
}

/// Extension trait for [`AsyncSeekForward`].
pub trait AsyncSeekForwardExt: AsyncSeekForward {
    /// Seek by the provided `offset` in the forwards direction, using the [`AsyncSeekForward`] trait.
    fn seek_forward(&mut self, offset: u64) -> SeekForwardFuture<'_, Self>
    where
        Self: Unpin,
    {
        SeekForwardFuture {
            seeker: self,
            offset,
        }
    }
}

impl<R: AsyncSeekForward + ?Sized> AsyncSeekForwardExt for R {}

#[derive(Debug)]
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct SeekForwardFuture<'a, S: Unpin + ?Sized> {
    seeker: &'a mut S,
    offset: u64,
}

impl<S: Unpin + ?Sized> Unpin for SeekForwardFuture<'_, S> {}

impl<S: AsyncSeekForward + Unpin + ?Sized> Future for SeekForwardFuture<'_, S> {
    type Output = futures_lite::io::Result<u64>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let offset = self.offset;
        Pin::new(&mut *self.seeker).poll_seek_forward(cx, offset)
    }
}

/// A type returned from [`AssetReader::read`], which is used to read the contents of a file
/// (or virtual file) corresponding to an asset.
///
/// This is essentially a trait alias for types implementing [`AsyncRead`] and [`AsyncSeekForward`].
/// The only reason a blanket implementation is not provided for applicable types is to allow
/// implementors to override the provided implementation of [`Reader::read_to_end`].
pub trait Reader: AsyncRead + AsyncSeekForward + Unpin + Send + Sync {
    /// Reads the entire contents of this reader and appends them to a vec.
    ///
    /// # Note for implementors
    /// You should override the provided implementation if you can fill up the buffer more
    /// efficiently than the default implementation, which calls `poll_read` repeatedly to
    /// fill up the buffer 32 bytes at a time.
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        let future = futures_lite::AsyncReadExt::read_to_end(self, buf);
        StackFuture::from(future)
    }
}

impl Reader for Box<dyn Reader + '_> {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        (**self).read_to_end(buf)
    }
}

/// A future that returns a value or an [`AssetReaderError`]
pub trait AssetReaderFuture:
    ConditionalSendFuture<Output = Result<Self::Value, AssetReaderError>>
{
    type Value;
}

impl<F, T> AssetReaderFuture for F
where
    F: ConditionalSendFuture<Output = Result<T, AssetReaderError>>,
{
    type Value = T;
}

/// Performs read operations on an asset storage. [`AssetReader`] exposes a "virtual filesystem"
/// API, where asset bytes and asset metadata bytes are both stored and accessible for a given
/// `path`. This trait is not object safe, if needed use a dyn [`ErasedAssetReader`] instead.
///
/// This trait defines asset-agnostic mechanisms to read bytes from a storage system.
/// For the per-asset-type saving/loading logic, see [`AssetSaver`](crate::saver::AssetSaver) and [`AssetLoader`](crate::loader::AssetLoader).
///
/// For a complementary version of this trait that can write assets to storage, see [`AssetWriter`].
pub trait AssetReader: Send + Sync + 'static {
    /// Returns a future to load the full file data at the provided path.
    ///
    /// # Note for implementors
    /// The preferred style for implementing this method is an `async fn` returning an opaque type.
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use bevy_asset::{prelude::*, io::{AssetReader, PathStream, Reader, AssetReaderError}};
    /// # struct MyReader;
    /// impl AssetReader for MyReader {
    ///     async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
    ///         // ...
    ///         # let val: Box<dyn Reader> = unimplemented!(); Ok(val)
    ///     }
    ///     # async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
    ///     #     let val: Box<dyn Reader> = unimplemented!(); Ok(val) }
    ///     # async fn read_directory<'a>(&'a self, path: &'a Path) -> Result<Box<PathStream>, AssetReaderError> { unimplemented!() }
    ///     # async fn is_directory<'a>(&'a self, path: &'a Path) -> Result<bool, AssetReaderError> { unimplemented!() }
    ///     # async fn read_meta_bytes<'a>(&'a self, path: &'a Path) -> Result<Vec<u8>, AssetReaderError> { unimplemented!() }
    /// }
    /// ```
    fn read<'a>(&'a self, path: &'a Path) -> impl AssetReaderFuture<Value: Reader + 'a>;
    /// Returns a future to load the full file data at the provided path.
    fn read_meta<'a>(&'a self, path: &'a Path) -> impl AssetReaderFuture<Value: Reader + 'a>;
    /// Returns an iterator of directory entry names at the provided path.
    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<PathStream>, AssetReaderError>>;
    /// Returns true if the provided path points to a directory.
    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<bool, AssetReaderError>>;
    /// Reads asset metadata bytes at the given `path` into a [`Vec<u8>`]. This is a convenience
    /// function that wraps [`AssetReader::read_meta`] by default.
    fn read_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Vec<u8>, AssetReaderError>> {
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
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>>;
    /// Returns a future to load the full file data at the provided path.
    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>>;
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
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>> {
        Box::pin(async {
            let reader = Self::read(self, path).await?;
            Ok(Box::new(reader) as Box<dyn Reader>)
        })
    }
    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<dyn Reader + 'a>, AssetReaderError>> {
        Box::pin(async {
            let reader = Self::read_meta(self, path).await?;
            Ok(Box::new(reader) as Box<dyn Reader>)
        })
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
/// `path`. This trait is not object safe, if needed use a dyn [`ErasedAssetWriter`] instead.
///
/// This trait defines asset-agnostic mechanisms to write bytes to a storage system.
/// For the per-asset-type saving/loading logic, see [`AssetSaver`](crate::saver::AssetSaver) and [`AssetLoader`](crate::loader::AssetLoader).
///
/// For a complementary version of this trait that can read assets from storage, see [`AssetReader`].
pub trait AssetWriter: Send + Sync + 'static {
    /// Writes the full asset bytes at the provided path.
    fn write<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<Writer>, AssetWriterError>>;
    /// Writes the full asset meta bytes at the provided path.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn write_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<Box<Writer>, AssetWriterError>>;
    /// Removes the asset stored at the given path.
    fn remove<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Removes the asset meta stored at the given path.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn remove_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Renames the asset at `old_path` to `new_path`
    fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Renames the asset meta for the asset at `old_path` to `new_path`.
    /// This _should not_ include storage specific extensions like `.meta`.
    fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Creates a directory at the given path, including all parent directories if they do not
    /// already exist.
    fn create_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Removes the directory at the given path, including all assets _and_ directories in that directory.
    fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Removes the directory at the given path, but only if it is completely empty. This will return an error if the
    /// directory is not empty.
    fn remove_empty_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Removes all assets (and directories) in this directory, resulting in an empty directory.
    fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>>;
    /// Writes the asset `bytes` to the given `path`.
    fn write_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>> {
        async {
            let mut writer = self.write(path).await?;
            writer.write_all(bytes).await?;
            writer.flush().await?;
            Ok(())
        }
    }
    /// Writes the asset meta `bytes` to the given `path`.
    fn write_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> impl ConditionalSendFuture<Output = Result<(), AssetWriterError>> {
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
    /// Creates a directory at the given path, including all parent directories if they do not
    /// already exist.
    fn create_directory<'a>(
        &'a self,
        path: &'a Path,
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
    fn create_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::create_directory(self, path))
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
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
        Box::pin(Self::write_bytes(self, path, bytes))
    }
    fn write_meta_bytes<'a>(
        &'a self,
        path: &'a Path,
        bytes: &'a [u8],
    ) -> BoxedFuture<'a, Result<(), AssetWriterError>> {
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
        cx: &mut Context<'_>,
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

impl AsyncSeekForward for VecReader {
    fn poll_seek_forward(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        offset: u64,
    ) -> Poll<std::io::Result<u64>> {
        let result = self
            .bytes_read
            .try_into()
            .map(|bytes_read: u64| bytes_read + offset);

        if let Ok(new_pos) = result {
            self.bytes_read = new_pos as _;
            Poll::Ready(Ok(new_pos as _))
        } else {
            Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek position is out of range",
            )))
        }
    }
}

impl Reader for VecReader {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        StackFuture::from(async {
            if self.bytes_read >= self.bytes.len() {
                Ok(0)
            } else {
                buf.extend_from_slice(&self.bytes[self.bytes_read..]);
                let n = self.bytes.len() - self.bytes_read;
                self.bytes_read = self.bytes.len();
                Ok(n)
            }
        })
    }
}

/// An [`AsyncRead`] implementation capable of reading a [`&[u8]`].
pub struct SliceReader<'a> {
    bytes: &'a [u8],
    bytes_read: usize,
}

impl<'a> SliceReader<'a> {
    /// Create a new [`SliceReader`] for `bytes`.
    pub fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            bytes_read: 0,
        }
    }
}

impl<'a> AsyncRead for SliceReader<'a> {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        if self.bytes_read >= self.bytes.len() {
            Poll::Ready(Ok(0))
        } else {
            let n = ready!(Pin::new(&mut &self.bytes[self.bytes_read..]).poll_read(cx, buf))?;
            self.bytes_read += n;
            Poll::Ready(Ok(n))
        }
    }
}

impl<'a> AsyncSeekForward for SliceReader<'a> {
    fn poll_seek_forward(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        offset: u64,
    ) -> Poll<std::io::Result<u64>> {
        let result = self
            .bytes_read
            .try_into()
            .map(|bytes_read: u64| bytes_read + offset);

        if let Ok(new_pos) = result {
            self.bytes_read = new_pos as _;

            Poll::Ready(Ok(new_pos as _))
        } else {
            Poll::Ready(Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "seek position is out of range",
            )))
        }
    }
}

impl Reader for SliceReader<'_> {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        StackFuture::from(async {
            if self.bytes_read >= self.bytes.len() {
                Ok(0)
            } else {
                buf.extend_from_slice(&self.bytes[self.bytes_read..]);
                let n = self.bytes.len() - self.bytes_read;
                self.bytes_read = self.bytes.len();
                Ok(n)
            }
        })
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

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
/// A [`PathBuf`] [`Stream`] implementation that immediately returns nothing.
struct EmptyPathStream;

#[cfg(any(target_arch = "wasm32", target_os = "android"))]
impl Stream for EmptyPathStream {
    type Item = PathBuf;

    fn poll_next(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Poll::Ready(None)
    }
}
