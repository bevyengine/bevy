use std::{
    io::{Error, ErrorKind},
    path::{Path, PathBuf},
};

use bevy_ecs::{system::Resource, world::World};
use bevy_utils::Duration;

use crate::io::{
    AssetReader, AssetSource, AssetSourceBuilder, AssetSourceEvent, AssetWatcher, AssetWriter,
    ErasedAssetReader, ErasedAssetWriter,
};

/// A [resource](`Resource`) providing access to the temporary directory used by the `temp://`
/// [asset source](`AssetSource`).
#[derive(Resource)]
pub struct TempDirectory {
    directory: TempDirectoryKind,
}

impl TempDirectory {
    /// Try to create a new [`TempDirectory`] resource, which uses a randomly created
    /// directory in the user's temporary directory. This can fail if the platform does not
    /// provide an appropriate temporary directory, or the directory itself could not be created.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_transient() -> std::io::Result<Self> {
        let directory = TempDirectoryKind::new_transient()?;

        Ok(Self { directory })
    }

    /// Create a new [`TempDirectory`] resource, which uses a provided directory to store temporary
    /// assets. It is assumed this directory already exists, and it will _not_ be deleted on exit.
    pub fn new_persistent(path: impl Into<PathBuf>) -> Self {
        let directory = TempDirectoryKind::new_persistent(path);

        Self { directory }
    }

    /// Get the [`Path`] to the directory used for temporary assets.
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Persist the current temporary asset directory after application exit.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn persist(&mut self) -> &mut Self {
        self.directory.persist();

        self
    }
}

/// Private resource to store the temporary directory used by `temp://`.
/// Kept private as it should only be removed on application exit.
enum TempDirectoryKind {
    /// Uses [`TempDir`](tempfile::TempDir)'s drop behavior to delete the directory.
    /// Note that this is not _guaranteed_ to succeed, so it is possible to leak files from this
    /// option until the underlying OS cleans temporary directories. For secure files, consider using
    /// [`tempfile`](tempfile::tempfile) directly.
    #[cfg(not(target_arch = "wasm32"))]
    Delete(tempfile::TempDir),
    /// Will not delete the temporary directory on exit, leaving cleanup the responsibility of
    /// the user or their system.
    Persist(PathBuf),
}

impl TempDirectoryKind {
    #[cfg(not(target_arch = "wasm32"))]
    fn new_transient() -> std::io::Result<Self> {
        let directory = tempfile::TempDir::with_prefix("bevy_")?;
        Ok(Self::Delete(directory))
    }

    fn new_persistent(path: impl Into<PathBuf>) -> Self {
        Self::Persist(path.into())
    }

    fn path(&self) -> &Path {
        match self {
            #[cfg(not(target_arch = "wasm32"))]
            Self::Delete(x) => x.as_ref(),
            Self::Persist(x) => x.as_ref(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn persist(&mut self) -> &mut Self {
        let mut swap = Self::Persist(PathBuf::new());

        std::mem::swap(self, &mut swap);

        let new = match swap {
            Self::Delete(x) => Self::Persist(x.into_path()),
            x @ Self::Persist(_) => x,
        };

        *self = new;

        self
    }
}

pub(crate) fn get_temp_source(
    world: &mut World,
    temporary_file_path: Option<String>,
) -> std::io::Result<AssetSourceBuilder> {
    let temp_dir = match world.remove_resource::<TempDirectory>() {
        Some(resource) => resource,
        None => match temporary_file_path {
            Some(path) => TempDirectory::new_persistent(path),
            None => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    TempDirectory::new_transient()?
                }

                #[cfg(target_arch = "wasm32")]
                {
                    TempDirectory::new_persistent("bevy_temp")
                }
            }
        },
    };

    let path: &str = temp_dir
        .path()
        .as_os_str()
        .try_into()
        .map_err(|error| Error::new(ErrorKind::InvalidData, error))?;

    let path = path.to_owned();
    let debounce = Duration::from_millis(300);

    let source = AssetSourceBuilder::default()
        .with_reader(TempAssetReader::get_default(path.clone()))
        .with_writer(TempAssetWriter::get_default(path.clone()))
        .with_watcher(TempAssetWatcher::get_default(path.clone(), debounce))
        .with_watch_warning(TempAssetWatcher::get_default_watch_warning());

    world.insert_resource(temp_dir);

    Ok(source)
}

struct TempAssetReader {
    inner: Box<dyn ErasedAssetReader>,
}

impl TempAssetReader {
    fn get_default(path: String) -> impl FnMut() -> Box<dyn ErasedAssetReader> + Send + Sync {
        move || {
            let inner = {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut getter = AssetSource::get_default_reader(path.clone());
                    getter()
                }

                #[cfg(target_arch = "wasm32")]
                {
                    Box::new(crate::io::wasm::OriginPrivateFileSystem::new(
                        path.clone().into(),
                    ))
                }
            };

            Box::new(Self { inner })
        }
    }
}

impl AssetReader for TempAssetReader {
    async fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<crate::io::Reader<'a>>, crate::io::AssetReaderError> {
        self.inner.read(path).await
    }

    async fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<crate::io::Reader<'a>>, crate::io::AssetReaderError> {
        self.inner.read_meta(path).await
    }

    async fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<crate::io::PathStream>, crate::io::AssetReaderError> {
        self.inner.read_directory(path).await
    }

    async fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<bool, crate::io::AssetReaderError> {
        self.inner.is_directory(path).await
    }
}

struct TempAssetWriter {
    inner: Box<dyn ErasedAssetWriter>,
}

impl TempAssetWriter {
    fn get_default(
        path: String,
    ) -> impl FnMut(bool) -> Option<Box<dyn ErasedAssetWriter>> + Send + Sync {
        move |_condition| {
            let inner = {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let mut getter = AssetSource::get_default_writer(path.clone());
                    getter(_condition)?
                }

                #[cfg(target_arch = "wasm32")]
                {
                    Box::new(crate::io::wasm::OriginPrivateFileSystem::new(
                        path.clone().into(),
                    ))
                }
            };

            Some(Box::new(Self { inner }))
        }
    }
}

impl AssetWriter for TempAssetWriter {
    async fn write<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<crate::io::Writer>, crate::io::AssetWriterError> {
        self.inner.write(path).await
    }

    async fn write_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<Box<crate::io::Writer>, crate::io::AssetWriterError> {
        self.inner.write_meta(path).await
    }

    async fn remove<'a>(&'a self, path: &'a Path) -> Result<(), crate::io::AssetWriterError> {
        self.inner.remove(path).await
    }

    async fn remove_meta<'a>(&'a self, path: &'a Path) -> Result<(), crate::io::AssetWriterError> {
        self.inner.remove_meta(path).await
    }

    async fn rename<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> Result<(), crate::io::AssetWriterError> {
        self.inner.rename(old_path, new_path).await
    }

    async fn rename_meta<'a>(
        &'a self,
        old_path: &'a Path,
        new_path: &'a Path,
    ) -> Result<(), crate::io::AssetWriterError> {
        self.inner.rename_meta(old_path, new_path).await
    }

    async fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<(), crate::io::AssetWriterError> {
        self.inner.remove_directory(path).await
    }

    async fn remove_empty_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<(), crate::io::AssetWriterError> {
        self.inner.remove_empty_directory(path).await
    }

    async fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> Result<(), crate::io::AssetWriterError> {
        self.inner.remove_assets_in_directory(path).await
    }
}

struct TempAssetWatcher {
    _inner: Box<dyn AssetWatcher>,
}

impl TempAssetWatcher {
    fn get_default(
        path: String,
        file_debounce_wait_time: Duration,
    ) -> impl FnMut(crossbeam_channel::Sender<AssetSourceEvent>) -> Option<Box<dyn AssetWatcher>>
           + Send
           + Sync {
        move |channel| {
            let mut getter =
                AssetSource::get_default_watcher(path.clone(), file_debounce_wait_time);
            let _inner = getter(channel)?;

            Some(Box::new(Self { _inner }))
        }
    }

    fn get_default_watch_warning() -> &'static str {
        AssetSource::get_default_watch_warning()
    }
}

impl AssetWatcher for TempAssetWatcher {}
