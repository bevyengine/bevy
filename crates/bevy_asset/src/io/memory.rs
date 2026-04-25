use crate::{
    io::{
        AssetReader, AssetReaderError, AssetWriter, AssetWriterError, PathStream, Reader,
        ReaderNotSeekableError, SeekableReader,
    },
    join_paths, path_components, split_path,
};
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use bevy_platform::{
    collections::HashMap,
    sync::{PoisonError, RwLock},
};
use core::{pin::Pin, task::Poll};
use futures_io::{AsyncRead, AsyncWrite};
use futures_lite::Stream;
use std::io::{Error, ErrorKind, SeekFrom};

use super::AsyncSeek;

#[derive(Default, Debug)]
struct DirInternal {
    assets: HashMap<Box<str>, Data>,
    metadata: HashMap<Box<str>, Data>,
    dirs: HashMap<Box<str>, Dir>,
    path: String,
}

/// A clone-able (internally Arc-ed) / thread-safe "in memory" filesystem.
/// This is built for [`MemoryAssetReader`] and is primarily intended for unit tests.
#[derive(Default, Clone, Debug)]
pub struct Dir(Arc<RwLock<DirInternal>>);

impl Dir {
    /// Creates a new [`Dir`] for the given `path`.
    pub fn new(path: String) -> Self {
        Self(Arc::new(RwLock::new(DirInternal {
            path,
            ..Default::default()
        })))
    }

    pub fn insert_asset_text(&self, path: &str, asset: &str) {
        self.insert_asset(path, asset.as_bytes().to_vec());
    }

    pub fn insert_meta_text(&self, path: &str, asset: &str) {
        self.insert_meta(path, asset.as_bytes().to_vec());
    }

    pub fn insert_asset(&self, path: &str, value: impl Into<Value>) {
        let mut dir = self.clone();
        let (parent, basename) = split_path(path);
        if let Some(parent) = parent {
            dir = self.get_or_insert_dir(parent);
        }
        dir.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .assets
            .insert(
                basename.unwrap().into(),
                Data {
                    value: value.into(),
                    path: path.to_string(),
                },
            );
    }

    /// Removes the stored asset at `path`.
    ///
    /// Returns the [`Data`] stored if found, [`None`] otherwise.
    pub fn remove_asset(&self, path: &str) -> Option<Data> {
        let mut dir = self.clone();
        let (parent, basename) = split_path(path);
        if let Some(parent) = parent {
            dir = self.get_or_insert_dir(parent);
        }
        let key: Box<str> = basename.unwrap().into();
        dir.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .assets
            .remove(&key)
    }

    pub fn insert_meta(&self, path: &str, value: impl Into<Value>) {
        let mut dir = self.clone();
        let (parent, basename) = split_path(path);
        if let Some(parent) = parent {
            dir = self.get_or_insert_dir(parent);
        }
        dir.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .metadata
            .insert(
                basename.unwrap().into(),
                Data {
                    value: value.into(),
                    path: path.to_string(),
                },
            );
    }

    /// Removes the stored metadata at `path`.
    ///
    /// Returns the [`Data`] stored if found, [`None`] otherwise.
    pub fn remove_metadata(&self, path: &str) -> Option<Data> {
        let mut dir = self.clone();
        let (parent, basename) = split_path(path);
        if let Some(parent) = parent {
            dir = self.get_or_insert_dir(parent);
        }
        let key: Box<str> = basename.unwrap().into();
        dir.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .metadata
            .remove(&key)
    }

    pub fn get_or_insert_dir(&self, path: &str) -> Dir {
        let mut dir = self.clone();
        let mut full_path = String::new();
        for name in path_components(path) {
            full_path.push('/');
            full_path.push_str(name);
            dir = {
                let dirs = &mut dir.0.write().unwrap_or_else(PoisonError::into_inner).dirs;
                dirs.entry(name.into())
                    .or_insert_with(|| Dir::new(full_path.clone()))
                    .clone()
            };
        }

        dir
    }

    /// Removes the dir at `path`.
    ///
    /// Returns the [`Dir`] stored if found, [`None`] otherwise.
    pub fn remove_dir(&self, path: &str) -> Option<Dir> {
        let mut dir = self.clone();
        let (parent, basename) = split_path(path);
        if let Some(parent) = parent {
            dir = self.get_or_insert_dir(parent);
        }
        let key: Box<str> = basename.unwrap().into();
        dir.0
            .write()
            .unwrap_or_else(PoisonError::into_inner)
            .dirs
            .remove(&key)
    }

    pub fn get_dir(&self, path: &str) -> Option<Dir> {
        let mut dir = self.clone();
        for name in path_components(path) {
            let next_dir = dir
                .0
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .dirs
                .get(name)?
                .clone();
            dir = next_dir;
        }
        Some(dir)
    }

    pub fn get_asset(&self, path: &str) -> Option<Data> {
        let mut dir = self.clone();
        let (parent, basename) = split_path(path);
        if let Some(parent) = parent {
            dir = dir.get_dir(parent)?;
        }

        basename.and_then(|f| {
            dir.0
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .assets
                .get(f)
                .cloned()
        })
    }

    pub fn get_metadata(&self, path: &str) -> Option<Data> {
        let mut dir = self.clone();
        let (parent, basename) = split_path(path);
        if let Some(parent) = parent {
            dir = dir.get_dir(parent)?;
        }

        basename.and_then(|f| {
            dir.0
                .read()
                .unwrap_or_else(PoisonError::into_inner)
                .metadata
                .get(f)
                .cloned()
        })
    }

    pub fn path(&self) -> String {
        self.0
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .path
            .to_string()
    }
}

pub struct DirStream {
    dir: Dir,
    index: usize,
    dir_index: usize,
}

impl DirStream {
    fn new(dir: Dir) -> Self {
        Self {
            dir,
            index: 0,
            dir_index: 0,
        }
    }
}

impl Stream for DirStream {
    type Item = String;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let dir = this.dir.0.read().unwrap_or_else(PoisonError::into_inner);

        let dir_index = this.dir_index;
        if let Some(dir_path) = dir
            .dirs
            .keys()
            .nth(dir_index)
            .map(|d| join_paths(&dir.path, d.as_ref()))
        {
            this.dir_index += 1;
            Poll::Ready(Some(dir_path))
        } else {
            let index = this.index;
            this.index += 1;
            Poll::Ready(dir.assets.values().nth(index).map(|d| d.path().to_owned()))
        }
    }
}

/// In-memory [`AssetReader`] implementation.
/// This is primarily intended for unit tests.
#[derive(Default, Clone)]
pub struct MemoryAssetReader {
    pub root: Dir,
}

/// In-memory [`AssetWriter`] implementation.
///
/// This is primarily intended for unit tests.
#[derive(Default, Clone)]
pub struct MemoryAssetWriter {
    pub root: Dir,
}

/// Asset data stored in a [`Dir`].
#[derive(Clone, Debug)]
pub struct Data {
    path: String,
    value: Value,
}

/// Stores either an allocated vec of bytes or a static array of bytes.
#[derive(Clone, Debug)]
pub enum Value {
    Vec(Arc<Vec<u8>>),
    Static(&'static [u8]),
}

impl Data {
    /// The path that this data was written to.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The value in bytes that was written here.
    pub fn value(&self) -> &[u8] {
        match &self.value {
            Value::Vec(vec) => vec,
            Value::Static(value) => value,
        }
    }
}

impl From<Vec<u8>> for Value {
    fn from(value: Vec<u8>) -> Self {
        Self::Vec(Arc::new(value))
    }
}

impl From<&'static [u8]> for Value {
    fn from(value: &'static [u8]) -> Self {
        Self::Static(value)
    }
}

impl<const N: usize> From<&'static [u8; N]> for Value {
    fn from(value: &'static [u8; N]) -> Self {
        Self::Static(value)
    }
}

struct DataReader {
    data: Data,
    bytes_read: usize,
}

impl AsyncRead for DataReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_io::Result<usize>> {
        // Get the mut borrow to avoid trying to borrow the pin itself multiple times.
        let this = self.get_mut();
        Poll::Ready(Ok(crate::io::slice_read(
            this.data.value(),
            &mut this.bytes_read,
            buf,
        )))
    }
}

impl AsyncSeek for DataReader {
    fn poll_seek(
        self: Pin<&mut Self>,
        _cx: &mut core::task::Context<'_>,
        pos: SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        // Get the mut borrow to avoid trying to borrow the pin itself multiple times.
        let this = self.get_mut();
        Poll::Ready(crate::io::slice_seek(
            this.data.value(),
            &mut this.bytes_read,
            pos,
        ))
    }
}

impl Reader for DataReader {
    fn read_to_end<'a>(
        &'a mut self,
        buf: &'a mut Vec<u8>,
    ) -> stackfuture::StackFuture<'a, std::io::Result<usize>, { super::STACK_FUTURE_SIZE }> {
        crate::io::read_to_end(self.data.value(), &mut self.bytes_read, buf)
    }

    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        Ok(self)
    }
}

impl AssetReader for MemoryAssetReader {
    async fn read<'a>(&'a self, path: &'a str) -> Result<impl Reader + 'a, AssetReaderError> {
        self.root
            .get_asset(path)
            .map(|data| DataReader {
                data,
                bytes_read: 0,
            })
            .ok_or_else(|| AssetReaderError::NotFound(path.to_string()))
    }

    async fn read_meta<'a>(&'a self, path: &'a str) -> Result<impl Reader + 'a, AssetReaderError> {
        self.root
            .get_metadata(path)
            .map(|data| DataReader {
                data,
                bytes_read: 0,
            })
            .ok_or_else(|| AssetReaderError::NotFound(path.to_string()))
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        self.root
            .get_dir(path)
            .map(|dir| {
                let stream: Box<PathStream> = Box::new(DirStream::new(dir));
                stream
            })
            .ok_or_else(|| AssetReaderError::NotFound(path.to_string()))
    }

    async fn is_directory<'a>(&'a self, path: &'a str) -> Result<bool, AssetReaderError> {
        Ok(self.root.get_dir(path).is_some())
    }
}

/// A writer that writes into [`Dir`], buffering internally until flushed/closed.
struct DataWriter {
    /// The dir to write to.
    dir: Dir,
    /// The path to write to.
    path: String,
    /// The current buffer of data.
    ///
    /// This will include data that has been flushed already.
    current_data: Vec<u8>,
    /// Whether to write to the data or to the meta.
    is_meta_writer: bool,
}

impl AsyncWrite for DataWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        self.get_mut().current_data.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _: &mut core::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        // Write the data to our fake disk. This means we will repeatedly reinsert the asset.
        if self.is_meta_writer {
            self.dir.insert_meta(&self.path, self.current_data.clone());
        } else {
            self.dir.insert_asset(&self.path, self.current_data.clone());
        }
        Poll::Ready(Ok(()))
    }

    fn poll_close(
        self: Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<std::io::Result<()>> {
        // A flush will just write the data to Dir, which is all we need to do for close.
        self.poll_flush(cx)
    }
}

impl AssetWriter for MemoryAssetWriter {
    async fn write<'a>(&'a self, path: &'a str) -> Result<Box<super::Writer>, AssetWriterError> {
        Ok(Box::new(DataWriter {
            dir: self.root.clone(),
            path: path.to_owned(),
            current_data: vec![],
            is_meta_writer: false,
        }))
    }

    async fn write_meta<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<Box<super::Writer>, AssetWriterError> {
        Ok(Box::new(DataWriter {
            dir: self.root.clone(),
            path: path.to_owned(),
            current_data: vec![],
            is_meta_writer: true,
        }))
    }

    async fn remove<'a>(&'a self, path: &'a str) -> Result<(), AssetWriterError> {
        if self.root.remove_asset(path).is_none() {
            return Err(AssetWriterError::Io(Error::new(
                ErrorKind::NotFound,
                "no such file",
            )));
        }
        Ok(())
    }

    async fn remove_meta<'a>(&'a self, path: &'a str) -> Result<(), AssetWriterError> {
        self.root.remove_metadata(path);
        Ok(())
    }

    async fn rename<'a>(
        &'a self,
        old_path: &'a str,
        new_path: &'a str,
    ) -> Result<(), AssetWriterError> {
        let Some(old_asset) = self.root.get_asset(old_path) else {
            return Err(AssetWriterError::Io(Error::new(
                ErrorKind::NotFound,
                "no such file",
            )));
        };
        self.root.insert_asset(new_path, old_asset.value);
        // Remove the asset after instead of before since otherwise there'd be a moment where the
        // Dir is unlocked and missing both the old and new paths. This just prevents race
        // conditions.
        self.root.remove_asset(old_path);
        Ok(())
    }

    async fn rename_meta<'a>(
        &'a self,
        old_path: &'a str,
        new_path: &'a str,
    ) -> Result<(), AssetWriterError> {
        let Some(old_meta) = self.root.get_metadata(old_path) else {
            return Err(AssetWriterError::Io(Error::new(
                ErrorKind::NotFound,
                "no such file",
            )));
        };
        self.root.insert_meta(new_path, old_meta.value);
        // Remove the meta after instead of before since otherwise there'd be a moment where the
        // Dir is unlocked and missing both the old and new paths. This just prevents race
        // conditions.
        self.root.remove_metadata(old_path);
        Ok(())
    }

    async fn create_directory<'a>(&'a self, path: &'a str) -> Result<(), AssetWriterError> {
        // Just pretend we're on a file system that doesn't consider directory re-creation a
        // failure.
        self.root.get_or_insert_dir(path);
        Ok(())
    }

    async fn remove_directory<'a>(&'a self, path: &'a str) -> Result<(), AssetWriterError> {
        if self.root.remove_dir(path).is_none() {
            return Err(AssetWriterError::Io(Error::new(
                ErrorKind::NotFound,
                "no such dir",
            )));
        }
        Ok(())
    }

    async fn remove_empty_directory<'a>(&'a self, path: &'a str) -> Result<(), AssetWriterError> {
        let Some(dir) = self.root.get_dir(path) else {
            return Err(AssetWriterError::Io(Error::new(
                ErrorKind::NotFound,
                "no such dir",
            )));
        };

        let dir = dir.0.read().unwrap();
        if !dir.assets.is_empty() || !dir.metadata.is_empty() || !dir.dirs.is_empty() {
            return Err(AssetWriterError::Io(Error::new(
                ErrorKind::DirectoryNotEmpty,
                "not empty",
            )));
        }

        self.root.remove_dir(path);
        Ok(())
    }

    async fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a str,
    ) -> Result<(), AssetWriterError> {
        let Some(dir) = self.root.get_dir(path) else {
            return Err(AssetWriterError::Io(Error::new(
                ErrorKind::NotFound,
                "no such dir",
            )));
        };

        let mut dir = dir.0.write().unwrap();
        dir.assets.clear();
        dir.dirs.clear();
        dir.metadata.clear();
        Ok(())
    }
}

#[cfg(test)]
pub mod test {
    use super::Dir;

    #[test]
    fn memory_dir() {
        let dir = Dir::default();
        let a_path = "a.txt";
        let a_data = "a".as_bytes().to_vec();
        let a_meta = "ameta".as_bytes().to_vec();

        dir.insert_asset(a_path, a_data.clone());
        let asset = dir.get_asset(a_path).unwrap();
        assert_eq!(asset.path(), a_path);
        assert_eq!(asset.value(), a_data);

        dir.insert_meta(a_path, a_meta.clone());
        let meta = dir.get_metadata(a_path).unwrap();
        assert_eq!(meta.path(), a_path);
        assert_eq!(meta.value(), a_meta);

        let b_path = "x/y/b.txt";
        let b_data = "b".as_bytes().to_vec();
        let b_meta = "meta".as_bytes().to_vec();
        dir.insert_asset(b_path, b_data.clone());
        dir.insert_meta(b_path, b_meta.clone());

        let asset = dir.get_asset(b_path).unwrap();
        assert_eq!(asset.path(), b_path);
        assert_eq!(asset.value(), b_data);

        let meta = dir.get_metadata(b_path).unwrap();
        assert_eq!(meta.path(), b_path);
        assert_eq!(meta.value(), b_meta);
    }
}
