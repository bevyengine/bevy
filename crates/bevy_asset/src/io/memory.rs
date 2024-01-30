use crate::io::{AssetReader, AssetReaderError, PathStream, Reader};
use bevy_utils::{BoxedFuture, HashMap};
use futures_io::AsyncRead;
use futures_lite::{ready, Stream};
use parking_lot::RwLock;
use std::{
    path::{Path, PathBuf},
    pin::Pin,
    sync::Arc,
    task::Poll,
};

#[derive(Default, Debug)]
struct DirInternal {
    assets: HashMap<String, Data>,
    metadata: HashMap<String, Data>,
    dirs: HashMap<String, Dir>,
    path: PathBuf,
}

/// A clone-able (internally Arc-ed) / thread-safe "in memory" filesystem.
/// This is built for [`MemoryAssetReader`] and is primarily intended for unit tests.
#[derive(Default, Clone, Debug)]
pub struct Dir(Arc<RwLock<DirInternal>>);

impl Dir {
    /// Creates a new [`Dir`] for the given `path`.
    pub fn new(path: PathBuf) -> Self {
        Self(Arc::new(RwLock::new(DirInternal {
            path,
            ..Default::default()
        })))
    }

    pub fn insert_asset_text(&self, path: &Path, asset: &str) {
        self.insert_asset(path, asset.as_bytes().to_vec());
    }

    pub fn insert_meta_text(&self, path: &Path, asset: &str) {
        self.insert_meta(path, asset.as_bytes().to_vec());
    }

    pub fn insert_asset(&self, path: &Path, value: impl Into<Value>) {
        let mut dir = self.clone();
        if let Some(parent) = path.parent() {
            dir = self.get_or_insert_dir(parent);
        }
        dir.0.write().assets.insert(
            path.file_name().unwrap().to_string_lossy().to_string(),
            Data {
                value: value.into(),
                path: path.to_owned(),
            },
        );
    }

    pub fn insert_meta(&self, path: &Path, value: impl Into<Value>) {
        let mut dir = self.clone();
        if let Some(parent) = path.parent() {
            dir = self.get_or_insert_dir(parent);
        }
        dir.0.write().metadata.insert(
            path.file_name().unwrap().to_string_lossy().to_string(),
            Data {
                value: value.into(),
                path: path.to_owned(),
            },
        );
    }

    pub fn get_or_insert_dir(&self, path: &Path) -> Dir {
        let mut dir = self.clone();
        let mut full_path = PathBuf::new();
        for c in path.components() {
            full_path.push(c);
            let name = c.as_os_str().to_string_lossy().to_string();
            dir = {
                let dirs = &mut dir.0.write().dirs;
                dirs.entry(name)
                    .or_insert_with(|| Dir::new(full_path.clone()))
                    .clone()
            };
        }

        dir
    }

    pub fn get_dir(&self, path: &Path) -> Option<Dir> {
        let mut dir = self.clone();
        for p in path.components() {
            let component = p.as_os_str().to_str().unwrap();
            let next_dir = dir.0.read().dirs.get(component)?.clone();
            dir = next_dir;
        }
        Some(dir)
    }

    pub fn get_asset(&self, path: &Path) -> Option<Data> {
        let mut dir = self.clone();
        if let Some(parent) = path.parent() {
            dir = dir.get_dir(parent)?;
        }

        path.file_name()
            .and_then(|f| dir.0.read().assets.get(f.to_str().unwrap()).cloned())
    }

    pub fn get_metadata(&self, path: &Path) -> Option<Data> {
        let mut dir = self.clone();
        if let Some(parent) = path.parent() {
            dir = dir.get_dir(parent)?;
        }

        path.file_name()
            .and_then(|f| dir.0.read().metadata.get(f.to_str().unwrap()).cloned())
    }

    pub fn path(&self) -> PathBuf {
        self.0.read().path.to_owned()
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
    type Item = PathBuf;

    fn poll_next(
        self: Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let dir = this.dir.0.read();

        let dir_index = this.dir_index;
        if let Some(dir_path) = dir.dirs.keys().nth(dir_index).map(|d| dir.path.join(d)) {
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

/// Asset data stored in a [`Dir`].
#[derive(Clone, Debug)]
pub struct Data {
    path: PathBuf,
    value: Value,
}

/// Stores either an allocated vec of bytes or a static array of bytes.
#[derive(Clone, Debug)]
pub enum Value {
    Vec(Arc<Vec<u8>>),
    Static(&'static [u8]),
}

impl Data {
    fn path(&self) -> &Path {
        &self.path
    }
    fn value(&self) -> &[u8] {
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
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<futures_io::Result<usize>> {
        if self.bytes_read >= self.data.value().len() {
            Poll::Ready(Ok(0))
        } else {
            let n =
                ready!(Pin::new(&mut &self.data.value()[self.bytes_read..]).poll_read(cx, buf))?;
            self.bytes_read += n;
            Poll::Ready(Ok(n))
        }
    }
}

impl AssetReader for MemoryAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            self.root
                .get_asset(path)
                .map(|data| {
                    let reader: Box<Reader> = Box::new(DataReader {
                        data,
                        bytes_read: 0,
                    });
                    reader
                })
                .ok_or_else(|| AssetReaderError::NotFound(path.to_path_buf()))
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            self.root
                .get_metadata(path)
                .map(|data| {
                    let reader: Box<Reader> = Box::new(DataReader {
                        data,
                        bytes_read: 0,
                    });
                    reader
                })
                .ok_or_else(|| AssetReaderError::NotFound(path.to_path_buf()))
        })
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move {
            self.root
                .get_dir(path)
                .map(|dir| {
                    let stream: Box<PathStream> = Box::new(DirStream::new(dir));
                    stream
                })
                .ok_or_else(|| AssetReaderError::NotFound(path.to_path_buf()))
        })
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<bool, AssetReaderError>> {
        Box::pin(async move { Ok(self.root.get_dir(path).is_some()) })
    }
}

#[cfg(test)]
pub mod test {
    use super::Dir;
    use std::path::Path;

    #[test]
    fn memory_dir() {
        let dir = Dir::default();
        let a_path = Path::new("a.txt");
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

        let b_path = Path::new("x/y/b.txt");
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
