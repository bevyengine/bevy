use crate::io::{
    AssetReader, AssetReaderError, AssetSourceEvent, AssetWatcher, AssetWriter, AssetWriterError,
    PathStream, Reader, Writer,
};
use anyhow::Result;
use async_fs::{read_dir, File};
use bevy_utils::BoxedFuture;
use crossbeam_channel::Sender;
use futures_lite::StreamExt;
use notify::{
    event::{CreateKind, ModifyKind, RemoveKind},
    Error, Event, RecommendedWatcher, RecursiveMode, Watcher,
};
use std::{
    env,
    path::{Path, PathBuf},
};

pub(crate) fn get_base_path() -> PathBuf {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        PathBuf::from(manifest_dir)
    } else {
        env::current_exe()
            .map(|path| {
                path.parent()
                    .map(|exe_parent_path| exe_parent_path.to_owned())
                    .unwrap()
            })
            .unwrap()
    }
}

/// I/O implementation for the local filesystem.
///
/// This asset I/O is fully featured but it's not available on `android` and `wasm` targets.
pub struct FileAssetReader {
    root_path: PathBuf,
}

impl FileAssetReader {
    /// Creates a new `FileAssetIo` at a path relative to the executable's directory, optionally
    /// watching for changes.
    ///
    /// See `get_base_path` below.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let root_path = Self::get_base_path().join(path.as_ref());
        std::fs::create_dir_all(&root_path).unwrap_or_else(|e| {
            panic!(
                "Failed to create root directory {:?} for file asset reader: {:?}",
                root_path, e
            )
        });
        Self { root_path }
    }

    /// Returns the base path of the assets directory, which is normally the executable's parent
    /// directory.
    ///
    /// If the `CARGO_MANIFEST_DIR` environment variable is set, then its value will be used
    /// instead. It's set by cargo when running with `cargo run`.
    pub fn get_base_path() -> PathBuf {
        get_base_path()
    }

    /// Returns the root directory where assets are loaded from.
    ///
    /// See `get_base_path`.
    pub fn root_path(&self) -> &PathBuf {
        &self.root_path
    }
}

fn get_meta_path(path: &Path) -> PathBuf {
    let mut meta_path = path.to_path_buf();
    let mut extension = path
        .extension()
        .expect("asset paths must have extensions")
        .to_os_string();
    extension.push(".meta");
    meta_path.set_extension(extension);
    meta_path
}

impl AssetReader for FileAssetReader {
    fn read<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        Box::pin(async move {
            let full_path = self.root_path.join(path);
            match File::open(&full_path).await {
                Ok(file) => {
                    let reader: Box<Reader> = Box::new(file);
                    Ok(reader)
                }
                Err(e) => {
                    return if e.kind() == std::io::ErrorKind::NotFound {
                        Err(AssetReaderError::NotFound(full_path))
                    } else {
                        Err(e.into())
                    }
                }
            }
        })
    }

    fn read_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Reader<'a>>, AssetReaderError>> {
        let meta_path = get_meta_path(path);
        Box::pin(async move {
            let full_path = self.root_path.join(meta_path);
            match File::open(&full_path).await {
                Ok(file) => {
                    let reader: Box<Reader> = Box::new(file);
                    Ok(reader)
                }
                Err(e) => {
                    return if e.kind() == std::io::ErrorKind::NotFound {
                        Err(AssetReaderError::NotFound(full_path))
                    } else {
                        Err(e.into())
                    }
                }
            }
        })
    }

    fn read_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<PathStream>, AssetReaderError>> {
        Box::pin(async move {
            let full_path = self.root_path.join(path);
            match read_dir(&full_path).await {
                Ok(read_dir) => {
                    let root_path = self.root_path.clone();
                    let mapped_stream = read_dir.filter_map(move |f| {
                        f.ok().and_then(|dir_entry| {
                            let path = dir_entry.path();
                            // filter out meta files as they are not considered assets
                            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                                if ext.eq_ignore_ascii_case("meta") {
                                    return None;
                                }
                            }
                            let relative_path = path.strip_prefix(&root_path).unwrap();
                            Some(relative_path.to_owned())
                        })
                    });
                    let read_dir: Box<PathStream> = Box::new(mapped_stream);
                    Ok(read_dir)
                }
                Err(e) => {
                    return if e.kind() == std::io::ErrorKind::NotFound {
                        Err(AssetReaderError::NotFound(full_path))
                    } else {
                        Err(e.into())
                    }
                }
            }
        })
    }

    fn is_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<bool, AssetReaderError>> {
        Box::pin(async move {
            let full_path = self.root_path.join(path);
            let metadata = full_path
                .metadata()
                .map_err(|_e| AssetReaderError::NotFound(path.to_owned()))?;
            Ok(metadata.file_type().is_dir())
        })
    }

    fn watch_for_changes(
        &self,
        event_sender: crossbeam_channel::Sender<super::AssetSourceEvent>,
    ) -> Option<Box<dyn AssetWatcher>> {
        Some(Box::new(
            FileWatcher::new(self.root_path.clone(), event_sender).unwrap(),
        ))
    }
}

pub struct FileAssetWriter {
    root_path: PathBuf,
}

impl FileAssetWriter {
    /// Creates a new `FileAssetIo` at a path relative to the executable's directory, optionally
    /// watching for changes.
    ///
    /// See `get_base_path` below.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            root_path: get_base_path().join(path.as_ref()),
        }
    }
}

impl AssetWriter for FileAssetWriter {
    fn write<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>> {
        Box::pin(async move {
            let full_path = self.root_path.join(path);
            if let Some(parent) = full_path.parent() {
                async_fs::create_dir_all(parent).await?;
            }
            let file = File::create(&full_path).await?;
            let reader: Box<Writer> = Box::new(file);
            Ok(reader)
        })
    }

    fn write_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, Result<Box<Writer>, AssetWriterError>> {
        Box::pin(async move {
            let meta_path = get_meta_path(path);
            let full_path = self.root_path.join(meta_path);
            if let Some(parent) = full_path.parent() {
                async_fs::create_dir_all(parent).await?;
            }
            let file = File::create(&full_path).await?;
            let reader: Box<Writer> = Box::new(file);
            Ok(reader)
        })
    }

    fn remove<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<(), AssetWriterError>> {
        Box::pin(async move {
            let full_path = self.root_path.join(path);
            async_fs::remove_file(full_path).await?;
            Ok(())
        })
    }

    fn remove_meta<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<(), AssetWriterError>> {
        Box::pin(async move {
            let meta_path = get_meta_path(path);
            let full_path = self.root_path.join(meta_path);
            async_fs::remove_file(full_path).await?;
            Ok(())
        })
    }

    fn remove_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<(), AssetWriterError>> {
        Box::pin(async move {
            let full_path = self.root_path.join(path);
            async_fs::remove_dir_all(full_path).await?;
            Ok(())
        })
    }

    fn remove_assets_in_directory<'a>(
        &'a self,
        path: &'a Path,
    ) -> BoxedFuture<'a, std::result::Result<(), AssetWriterError>> {
        Box::pin(async move {
            let full_path = self.root_path.join(path);
            async_fs::remove_dir_all(&full_path).await?;
            async_fs::create_dir_all(&full_path).await?;
            Ok(())
        })
    }
}

pub struct FileWatcher {
    _watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new(root: PathBuf, sender: Sender<AssetSourceEvent>) -> Result<Self, Error> {
        let owned_root = root.clone();
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, Error>| {
                let event = result.unwrap();
                match event.kind {
                    notify::EventKind::Create(CreateKind::File) => {
                        let (path, is_meta) = get_asset_path(&owned_root, &event.paths[0]);
                        if is_meta {
                            sender.send(AssetSourceEvent::AddedMeta(path)).unwrap();
                        } else {
                            sender.send(AssetSourceEvent::Added(path)).unwrap();
                        }
                    }
                    notify::EventKind::Create(CreateKind::Folder) => {
                        let (path, _) = get_asset_path(&owned_root, &event.paths[0]);
                        sender.send(AssetSourceEvent::AddedFolder(path)).unwrap();
                    }
                    notify::EventKind::Modify(ModifyKind::Data(_)) => {
                        let (path, is_meta) = get_asset_path(&owned_root, &event.paths[0]);
                        if is_meta {
                            sender.send(AssetSourceEvent::ModifiedMeta(path)).unwrap();
                        } else {
                            sender.send(AssetSourceEvent::Modified(path)).unwrap();
                        }
                    }
                    notify::EventKind::Remove(RemoveKind::File) => {
                        let (path, is_meta) = get_asset_path(&owned_root, &event.paths[0]);
                        if is_meta {
                            sender.send(AssetSourceEvent::RemovedMeta(path)).unwrap();
                        } else {
                            sender.send(AssetSourceEvent::Removed(path)).unwrap();
                        }
                    }
                    notify::EventKind::Remove(RemoveKind::Folder) => {
                        let (path, _) = get_asset_path(&owned_root, &event.paths[0]);
                        sender.send(AssetSourceEvent::RemovedFolder(path)).unwrap();
                    }
                    _ => {}
                }
            },
            notify::Config::default(),
        )?;
        watcher.watch(&root, RecursiveMode::Recursive)?;
        Ok(Self { _watcher: watcher })
    }
}

fn get_asset_path(root: &Path, absolute_path: &Path) -> (PathBuf, bool) {
    let relative_path = absolute_path.strip_prefix(root).unwrap();
    let is_meta = relative_path
        .extension()
        .map(|e| e == "meta")
        .unwrap_or(false);
    let asset_path = if is_meta {
        relative_path.with_extension("").to_owned()
    } else {
        relative_path.to_owned()
    };
    (asset_path, is_meta)
}

impl AssetWatcher for FileWatcher {}
