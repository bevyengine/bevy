use crate::{
    io::{AssetSourceEvent, AssetWatcher},
    path::normalize_path,
};
use alloc::{borrow::ToOwned, vec::Vec};
use async_channel::Sender;
use core::time::Duration;
use notify_debouncer_full::{
    new_debouncer,
    notify::{
        self,
        event::{AccessKind, AccessMode, CreateKind, ModifyKind, RemoveKind, RenameMode},
        RecommendedWatcher, RecursiveMode,
    },
    DebounceEventResult, Debouncer, RecommendedCache,
};
use std::path::{Path, PathBuf};
use tracing::error;

/// An [`AssetWatcher`] that watches the filesystem for changes to asset files in a given root folder and emits [`AssetSourceEvent`]
/// for each relevant change.
///
/// This uses [`notify_debouncer_full`] to retrieve "debounced" filesystem events.
/// "Debouncing" defines a time window to hold on to events and then removes duplicate events that fall into this window.
/// This introduces a small delay in processing events, but it helps reduce event duplicates. A small delay is also necessary
/// on some systems to avoid processing a change event before it has actually been applied.
pub struct FileWatcher {
    _watcher: Debouncer<RecommendedWatcher, RecommendedCache>,
}

impl FileWatcher {
    /// Creates a new [`FileWatcher`] that watches for changes to the asset files in the given `path`.
    pub fn new(
        path: PathBuf,
        sender: Sender<AssetSourceEvent>,
        debounce_wait_time: Duration,
    ) -> Result<Self, notify::Error> {
        let root = make_absolute_path(&path)?;
        let watcher = new_asset_event_debouncer(
            path.clone(),
            debounce_wait_time,
            FileEventHandler {
                root,
                sender,
                last_event: None,
            },
        )?;
        Ok(FileWatcher { _watcher: watcher })
    }
}

impl AssetWatcher for FileWatcher {}

/// Converts the provided path into an absolute one.
fn make_absolute_path(path: &Path) -> Result<PathBuf, std::io::Error> {
    // We use `normalize` + `absolute` instead of `canonicalize` to avoid reading the filesystem to
    // resolve the path. This also means that paths that no longer exist can still become absolute
    // (e.g., a file that was renamed will have the "old" path no longer exist).
    Ok(normalize_path(&std::path::absolute(path)?))
}

pub(crate) fn get_asset_path(root: &Path, absolute_path: &Path) -> (PathBuf, bool) {
    let relative_path = absolute_path.strip_prefix(root).unwrap_or_else(|_| {
        panic!(
            "FileWatcher::get_asset_path() failed to strip prefix from absolute path: absolute_path={}, root={}",
            absolute_path.display(),
            root.display()
        )
    });
    let is_meta = relative_path.extension().is_some_and(|e| e == "meta");
    let asset_path = if is_meta {
        relative_path.with_extension("")
    } else {
        relative_path.to_owned()
    };
    (asset_path, is_meta)
}

/// This is a bit more abstracted than it normally would be because we want to try _very hard_ not to duplicate this
/// event management logic across filesystem-driven [`AssetWatcher`] impls. Each operating system / platform behaves
/// a little differently and this is the result of a delicate balancing act that we should only perform once.
pub(crate) fn new_asset_event_debouncer(
    root: PathBuf,
    debounce_wait_time: Duration,
    mut handler: impl FilesystemEventHandler,
) -> Result<Debouncer<RecommendedWatcher, RecommendedCache>, notify::Error> {
    let root = super::get_base_path().join(root);
    let mut debouncer = new_debouncer(
        debounce_wait_time,
        None,
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    handler.begin();
                    for event in events.iter() {
                        // Make all the paths absolute here so we don't need to do it in each
                        // handler.
                        let paths = event
                            .paths
                            .iter()
                            .map(PathBuf::as_path)
                            .map(|p| {
                                make_absolute_path(p).expect("paths from the debouncer are valid")
                            })
                            .collect::<Vec<_>>();

                        match event.kind {
                            notify::EventKind::Create(CreateKind::File) => {
                                if let Some((path, is_meta)) = handler.get_path(&paths[0]) {
                                    if is_meta {
                                        handler.handle(&paths, AssetSourceEvent::AddedMeta(path));
                                    } else {
                                        handler.handle(&paths, AssetSourceEvent::AddedAsset(path));
                                    }
                                }
                            }
                            notify::EventKind::Create(CreateKind::Folder) => {
                                if let Some((path, _)) = handler.get_path(&paths[0]) {
                                    handler.handle(&paths, AssetSourceEvent::AddedFolder(path));
                                }
                            }
                            notify::EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                                if let Some((path, is_meta)) = handler.get_path(&paths[0]) {
                                    if is_meta {
                                        handler
                                            .handle(&paths, AssetSourceEvent::ModifiedMeta(path));
                                    } else {
                                        handler
                                            .handle(&paths, AssetSourceEvent::ModifiedAsset(path));
                                    }
                                }
                            }
                            // Because this is debounced over a reasonable period of time, Modify(ModifyKind::Name(RenameMode::From)
                            // events are assumed to be "dangling" without a follow up "To" event. Without debouncing, "From" -> "To" -> "Both"
                            // events are emitted for renames. If a From is dangling, it is assumed to be "removed" from the context of the asset
                            // system.
                            notify::EventKind::Remove(RemoveKind::Any)
                            | notify::EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                                if let Some((path, is_meta)) = handler.get_path(&paths[0]) {
                                    handler.handle(
                                        &paths,
                                        AssetSourceEvent::RemovedUnknown { path, is_meta },
                                    );
                                }
                            }
                            notify::EventKind::Create(CreateKind::Any)
                            | notify::EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                                if let Some((path, is_meta)) = handler.get_path(&paths[0]) {
                                    let asset_event = if paths[0].is_dir() {
                                        AssetSourceEvent::AddedFolder(path)
                                    } else if is_meta {
                                        AssetSourceEvent::AddedMeta(path)
                                    } else {
                                        AssetSourceEvent::AddedAsset(path)
                                    };
                                    handler.handle(&paths, asset_event);
                                }
                            }
                            notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                                let Some((old_path, old_is_meta)) = handler.get_path(&paths[0])
                                else {
                                    continue;
                                };
                                let Some((new_path, new_is_meta)) = handler.get_path(&paths[1])
                                else {
                                    continue;
                                };
                                // only the new "real" path is considered a directory
                                if paths[1].is_dir() {
                                    handler.handle(
                                        &paths,
                                        AssetSourceEvent::RenamedFolder {
                                            old: old_path,
                                            new: new_path,
                                        },
                                    );
                                } else {
                                    match (old_is_meta, new_is_meta) {
                                        (true, true) => {
                                            handler.handle(
                                                &paths,
                                                AssetSourceEvent::RenamedMeta {
                                                    old: old_path,
                                                    new: new_path,
                                                },
                                            );
                                        }
                                        (false, false) => {
                                            handler.handle(
                                                &paths,
                                                AssetSourceEvent::RenamedAsset {
                                                    old: old_path,
                                                    new: new_path,
                                                },
                                            );
                                        }
                                        (true, false) => {
                                            error!(
                                            "Asset metafile {old_path:?} was changed to asset file {new_path:?}, which is not supported. Try restarting your app to see if configuration is still valid"
                                        );
                                        }
                                        (false, true) => {
                                            error!(
                                            "Asset file {old_path:?} was changed to meta file {new_path:?}, which is not supported. Try restarting your app to see if configuration is still valid"
                                        );
                                        }
                                    }
                                }
                            }
                            notify::EventKind::Modify(_) => {
                                let Some((path, is_meta)) = handler.get_path(&paths[0]) else {
                                    continue;
                                };
                                if paths[0].is_dir() {
                                    // modified folder means nothing in this case
                                } else if is_meta {
                                    handler.handle(&paths, AssetSourceEvent::ModifiedMeta(path));
                                } else {
                                    handler.handle(&paths, AssetSourceEvent::ModifiedAsset(path));
                                };
                            }
                            notify::EventKind::Remove(RemoveKind::File) => {
                                let Some((path, is_meta)) = handler.get_path(&paths[0]) else {
                                    continue;
                                };
                                if is_meta {
                                    handler.handle(&paths, AssetSourceEvent::RemovedMeta(path));
                                } else {
                                    handler.handle(&paths, AssetSourceEvent::RemovedAsset(path));
                                }
                            }
                            notify::EventKind::Remove(RemoveKind::Folder) => {
                                let Some((path, _)) = handler.get_path(&paths[0]) else {
                                    continue;
                                };
                                handler.handle(&paths, AssetSourceEvent::RemovedFolder(path));
                            }
                            _ => {}
                        }
                    }
                }
                Err(errors) => errors.iter().for_each(|error| {
                    error!("Encountered a filesystem watcher error {error:?}");
                }),
            }
        },
    )?;
    debouncer.watch(&root, RecursiveMode::Recursive)?;
    Ok(debouncer)
}

pub(crate) struct FileEventHandler {
    sender: Sender<AssetSourceEvent>,
    root: PathBuf,
    last_event: Option<AssetSourceEvent>,
}

impl FilesystemEventHandler for FileEventHandler {
    fn begin(&mut self) {
        self.last_event = None;
    }
    fn get_path(&self, absolute_path: &Path) -> Option<(PathBuf, bool)> {
        Some(get_asset_path(&self.root, absolute_path))
    }

    fn handle(&mut self, _absolute_paths: &[PathBuf], event: AssetSourceEvent) {
        if self.last_event.as_ref() != Some(&event) {
            self.last_event = Some(event.clone());
            self.sender.send_blocking(event).unwrap();
        }
    }
}

pub(crate) trait FilesystemEventHandler: Send + Sync + 'static {
    /// Called each time a set of debounced events is processed
    fn begin(&mut self);
    /// Returns an actual asset path (if one exists for the given `absolute_path`), as well as a [`bool`] that is
    /// true if the `absolute_path` corresponds to a meta file.
    fn get_path(&self, absolute_path: &Path) -> Option<(PathBuf, bool)>;
    /// Handle the given event
    fn handle(&mut self, absolute_paths: &[PathBuf], event: AssetSourceEvent);
}
