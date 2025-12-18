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
pub mod memory;
pub mod processor_gated;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
#[cfg(any(feature = "http", feature = "https"))]
pub mod web;

#[cfg(test)]
pub mod gated;

mod source;

pub use futures_lite::AsyncWriteExt;
pub use source::*;

use alloc::{boxed::Box, sync::Arc, vec::Vec};
use bevy_tasks::{BoxedFuture, ConditionalSendFuture};
use core::{
    mem::size_of,
    pin::Pin,
    task::{Context, Poll},
};
use futures_io::{AsyncRead, AsyncSeek, AsyncWrite};
use futures_lite::Stream;
use std::{
    io::SeekFrom,
    path::{Path, PathBuf},
};
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
    /// - If the request returns a 404 error, expect [`AssetReaderError::NotFound`].
    /// - If the request fails before getting a status code (e.g. request timeout, interrupted connection, etc), expect [`AssetReaderError::Io`].
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

/// A type returned from [`AssetReader::read`], which is used to read the contents of a file
/// (or virtual file) corresponding to an asset.
///
/// This is essentially a trait alias for types implementing [`AsyncRead`] and [`AsyncSeek`].
/// The only reason a blanket implementation is not provided for applicable types is to allow
/// implementors to override the provided implementation of [`Reader::read_to_end`].
///
/// # Reader features
///
/// This trait includes super traits. However, this **does not** mean that your type needs to
/// support every feature of those super traits. If the caller never uses that feature, then a dummy
/// implementation that just returns an error is sufficient.
///
/// The caller can request a compatible [`Reader`] using [`ReaderRequiredFeatures`] (when using the
/// [`AssetReader`] trait). This allows the caller to state which features of the reader it will
/// use, avoiding cases where the caller uses a feature that the reader does not support.
///
/// For example, the caller may set [`ReaderRequiredFeatures::seek`] to
/// [`SeekKind::AnySeek`] to indicate that they may seek backward, or from the start/end. A reader
/// implementation may choose to support that, or may just detect those kinds of seeks and return an
/// error.
pub trait Reader: AsyncRead + Unpin + Send + Sync {
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

    /// Casts this [`Reader`] as a [`SeekableReader`], which layers on [`AsyncSeek`] functionality.
    /// Returns [`Ok`] if this [`Reader`] supports seeking. Otherwise returns [`Err`].
    ///
    /// Implementers of [`Reader`] are highly encouraged to provide this functionality, as it makes the
    /// reader compatible with "seeking" [`AssetLoader`](crate::AssetLoader) implementations.
    ///
    /// [`AssetLoader`](crate::AssetLoader) implementations that call this are encouraged to provide fallback behavior
    /// when it fails, such as reading into a seek-able [`Vec`] (or [`AsyncSeek`]-able [`VecReader`]):
    ///
    /// ```
    /// # use bevy_asset::io::{VecReader, Reader, AsyncSeekExt};
    /// # use std::{io::SeekFrom, vec::Vec};
    /// # let mut vec_reader = VecReader::new(Vec::new());
    /// # let reader: &mut dyn Reader = &mut vec_reader;
    /// let reader = match reader.seekable() {
    ///     Ok(seek) => seek,
    ///     Err(_) => {
    ///         reader.read_to_end(&mut data.bytes).await.unwrap();
    ///         &mut data
    ///     }
    /// };
    /// reader.seek(SeekFrom::Start(10)).await.unwrap();
    /// ```
    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError>;
}

pub trait SeekableReader: Reader + AsyncSeek {}

impl<T: Reader + AsyncSeek> SeekableReader for T {}

#[derive(Error, Debug, Copy, Clone)]
#[error(
    "The `Reader` returned by the current `AssetReader` does not support `AsyncSeek` behavior."
)]
pub struct ReaderNotSeekableError;

impl Reader for Box<dyn Reader + '_> {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        (**self).read_to_end(buf)
    }

    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        (**self).seekable()
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
    /// # Required Features
    ///
    /// The `required_features` allows the caller to request that the returned reader implements
    /// certain features, and consequently allows this trait to decide how to react to that request.
    /// Namely, the implementor could:
    ///
    /// * Return an error if the caller requests an unsupported feature. This can give a nicer error
    ///   message to make it clear that the caller (e.g., an asset loader) can't be used with this
    ///   reader.
    /// * Use a different implementation of a reader to ensure support of a feature (e.g., reading
    ///   the entire asset into memory and then providing that buffer as a reader).
    /// * Ignore the request and provide the regular reader anyway. Practically, if the caller never
    ///   actually uses the feature, it's fine to continue using the reader. However the caller
    ///   requesting a feature is a **strong signal** that they will use the given feature.
    ///
    /// The recommendation is to simply return an error for unsupported features. Callers can
    /// generally work around this and have more understanding of their constraints. For example,
    /// an asset loader may know that it will only load "small" assets, so reading the entire asset
    /// into memory won't consume too much memory, and so it can use the regular [`AsyncRead`] API
    /// to read the whole asset into memory. If this were done by this trait, the loader may
    /// accidentally be allocating too much memory for a large asset without knowing it!
    ///
    /// # Note for implementors
    /// The preferred style for implementing this method is an `async fn` returning an opaque type.
    ///
    /// ```no_run
    /// # use std::path::Path;
    /// # use bevy_asset::{prelude::*, io::{AssetReader, PathStream, Reader, AssetReaderError, ReaderRequiredFeatures}};
    /// # struct MyReader;
    /// impl AssetReader for MyReader {
    ///     async fn read<'a>(&'a self, path: &'a Path, required_features: ReaderRequiredFeatures) -> Result<impl Reader + 'a, AssetReaderError> {
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
        Box::pin(async move {
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
    pub bytes: Vec<u8>,
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
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_io::Result<usize>> {
        // Get the mut borrow to avoid trying to borrow the pin itself multiple times.
        let this = self.get_mut();
        Poll::Ready(Ok(slice_read(&this.bytes, &mut this.bytes_read, buf)))
    }
}

impl AsyncSeek for VecReader {
    fn poll_seek(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        // Get the mut borrow to avoid trying to borrow the pin itself multiple times.
        let this = self.get_mut();
        Poll::Ready(slice_seek(&this.bytes, &mut this.bytes_read, pos))
    }
}

impl Reader for VecReader {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        read_to_end(&self.bytes, &mut self.bytes_read, buf)
    }

    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Ok(self)
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
        _cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(slice_read(self.bytes, &mut self.bytes_read, buf)))
    }
}

impl<'a> AsyncSeek for SliceReader<'a> {
    fn poll_seek(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        Poll::Ready(slice_seek(self.bytes, &mut self.bytes_read, pos))
    }
}

impl Reader for SliceReader<'_> {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
        read_to_end(self.bytes, &mut self.bytes_read, buf)
    }

    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Ok(self)
    }
}

/// Performs a read from the `slice` into `buf`.
pub(crate) fn slice_read(slice: &[u8], bytes_read: &mut usize, buf: &mut [u8]) -> usize {
    if *bytes_read >= slice.len() {
        0
    } else {
        let n = std::io::Read::read(&mut &slice[(*bytes_read)..], buf).unwrap();
        *bytes_read += n;
        n
    }
}

/// Performs a "seek" and updates the cursor of `bytes_read`. Returns the new byte position.
pub(crate) fn slice_seek(
    slice: &[u8],
    bytes_read: &mut usize,
    pos: SeekFrom,
) -> std::io::Result<u64> {
    let make_error = || {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "seek position is out of range",
        ))
    };
    let (origin, offset) = match pos {
        SeekFrom::Current(offset) => (*bytes_read, Ok(offset)),
        SeekFrom::Start(offset) => (0, offset.try_into()),
        SeekFrom::End(offset) => (slice.len(), Ok(offset)),
    };
    let Ok(offset) = offset else {
        return make_error();
    };
    let Ok(origin): Result<i64, _> = origin.try_into() else {
        return make_error();
    };
    let Ok(new_pos) = (origin + offset).try_into() else {
        return make_error();
    };
    *bytes_read = new_pos;
    Ok(new_pos as _)
}

/// Copies bytes from source to dest, keeping track of where in the source it starts copying from.
///
/// This is effectively the impl for [`SliceReader::read_to_end`], but this is provided here so the
/// lifetimes are only tied to the buffer and not the [`SliceReader`] itself.
pub(crate) fn read_to_end<'a>(
    source: &'a [u8],
    bytes_read: &'a mut usize,
    dest: &'a mut Vec<u8>,
) -> StackFuture<'a, std::io::Result<usize>, STACK_FUTURE_SIZE> {
    StackFuture::from(async {
        if *bytes_read >= source.len() {
            Ok(0)
        } else {
            dest.extend_from_slice(&source[*bytes_read..]);
            let n = source.len() - *bytes_read;
            *bytes_read = source.len();
            Ok(n)
        }
    })
}

/// Appends `.meta` to the given path:
/// - `foo` becomes `foo.meta`
/// - `foo.bar` becomes `foo.bar.meta`
pub(crate) fn get_meta_path(path: &Path) -> PathBuf {
    let mut meta_path = path.to_path_buf();
    let mut extension = path.extension().unwrap_or_default().to_os_string();
    if !extension.is_empty() {
        extension.push(".");
    }
    extension.push("meta");
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_meta_path_no_extension() {
        assert_eq!(
            get_meta_path(Path::new("foo")).to_str().unwrap(),
            "foo.meta"
        );
    }

    #[test]
    fn get_meta_path_with_extension() {
        assert_eq!(
            get_meta_path(Path::new("foo.bar")).to_str().unwrap(),
            "foo.bar.meta"
        );
    }
}
