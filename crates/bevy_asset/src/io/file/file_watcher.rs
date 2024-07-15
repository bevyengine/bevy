use crate::io::{AssetSourceEvent, AssetWatcher};
use crate::path::normalize_path;
use bevy_utils::tracing::error;
use bevy_utils::Duration;
use crossbeam_channel::Sender;
use notify_debouncer_full::{
    new_debouncer,
    notify::{
        self,
        event::{AccessKind, AccessMode, CreateKind, ModifyKind, RemoveKind, RenameMode},
        RecommendedWatcher, RecursiveMode, Watcher,
    },
    DebounceEventResult, Debouncer, FileIdMap,
};
use std::path::{Path, PathBuf};

/// An [`AssetWatcher`] that watches the filesystem for changes to asset files in a given root folder and emits [`AssetSourceEvent`]
/// for each relevant change. This uses [`notify_debouncer_full`] to retrieve "debounced" filesystem events.
/// "Debouncing" defines a time window to hold on to events and then removes duplicate events that fall into this window.
/// This introduces a small delay in processing events, but it helps reduce event duplicates. A small delay is also necessary
/// on some systems to avoid processing a change event before it has actually been applied.
pub struct FileWatcher {
    _watcher: Debouncer<RecommendedWatcher, FileIdMap>,
}

impl FileWatcher {
    pub fn new(
        root: PathBuf,
        sender: Sender<AssetSourceEvent>,
        debounce_wait_time: Duration,
    ) -> Result<Self, notify::Error> {
        let root = normalize_path(super::get_base_path().join(root).as_path());
        let watcher = new_asset_event_debouncer(
            root.clone(),
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

pub(crate) fn get_asset_path(root: &Path, absolute_path: &Path) -> (PathBuf, bool) {
    let relative_path = absolute_path.strip_prefix(root).unwrap_or_else(|_| {
        panic!(
            "FileWatcher::get_asset_path() failed to strip prefix from absolute path: absolute_path={:?}, root={:?}",
            absolute_path,
            root
        )
    });
    let is_meta = relative_path
        .extension()
        .map(|e| e == "meta")
        .unwrap_or(false);
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
) -> Result<Debouncer<RecommendedWatcher, FileIdMap>, notify::Error> {
    let root = super::get_base_path().join(root);
    let mut debouncer = new_debouncer(
        debounce_wait_time,
        None,
        move |result: DebounceEventResult| {
            match result {
                Ok(events) => {
                    handler.begin();
                    for event in events.iter() {
                        match event.kind {
                            notify::EventKind::Create(CreateKind::File) => {
                                if let Some((path, is_meta)) = handler.get_path(&event.paths[0]) {
                                    if is_meta {
                                        handler.handle(
                                            &event.paths,
                                            AssetSourceEvent::AddedMeta(path),
                                        );
                                    } else {
                                        handler.handle(
                                            &event.paths,
                                            AssetSourceEvent::AddedAsset(path),
                                        );
                                    }
                                }
                            }
                            notify::EventKind::Create(CreateKind::Folder) => {
                                if let Some((path, _)) = handler.get_path(&event.paths[0]) {
                                    handler
                                        .handle(&event.paths, AssetSourceEvent::AddedFolder(path));
                                }
                            }
                            notify::EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                                if let Some((path, is_meta)) = handler.get_path(&event.paths[0]) {
                                    if is_meta {
                                        handler.handle(
                                            &event.paths,
                                            AssetSourceEvent::ModifiedMeta(path),
                                        );
                                    } else {
                                        handler.handle(
                                            &event.paths,
                                            AssetSourceEvent::ModifiedAsset(path),
                                        );
                                    }
                                }
                            }
                            // Because this is debounced over a reasonable period of time, Modify(ModifyKind::Name(RenameMode::From)
                            // events are assumed to be "dangling" without a follow up "To" event. Without debouncing, "From" -> "To" -> "Both"
                            // events are emitted for renames. If a From is dangling, it is assumed to be "removed" from the context of the asset
                            // system.
                            notify::EventKind::Remove(RemoveKind::Any)
                            | notify::EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                                if let Some((path, is_meta)) = handler.get_path(&event.paths[0]) {
                                    handler.handle(
                                        &event.paths,
                                        AssetSourceEvent::RemovedUnknown { path, is_meta },
                                    );
                                }
                            }
                            notify::EventKind::Create(CreateKind::Any)
                            | notify::EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                                if let Some((path, is_meta)) = handler.get_path(&event.paths[0]) {
                                    let asset_event = if event.paths[0].is_dir() {
                                        AssetSourceEvent::AddedFolder(path)
                                    } else if is_meta {
                                        AssetSourceEvent::AddedMeta(path)
                                    } else {
                                        AssetSourceEvent::AddedAsset(path)
                                    };
                                    handler.handle(&event.paths, asset_event);
                                }
                            }
                            notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                                let Some((old_path, old_is_meta)) =
                                    handler.get_path(&event.paths[0])
                                else {
                                    continue;
                                };
                                let Some((new_path, new_is_meta)) =
                                    handler.get_path(&event.paths[1])
                                else {
                                    continue;
                                };
                                // only the new "real" path is considered a directory
                                if event.paths[1].is_dir() {
                                    handler.handle(
                                        &event.paths,
                                        AssetSourceEvent::RenamedFolder {
                                            old: old_path,
                                            new: new_path,
                                        },
                                    );
                                } else {
                                    match (old_is_meta, new_is_meta) {
                                        (true, true) => {
                                            handler.handle(
                                                &event.paths,
                                                AssetSourceEvent::RenamedMeta {
                                                    old: old_path,
                                                    new: new_path,
                                                },
                                            );
                                        }
                                        (false, false) => {
                                            handler.handle(
                                                &event.paths,
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
                                let Some((path, is_meta)) = handler.get_path(&event.paths[0])
                                else {
                                    continue;
                                };
                                if event.paths[0].is_dir() {
                                    // modified folder means nothing in this case
                                } else if is_meta {
                                    handler
                                        .handle(&event.paths, AssetSourceEvent::ModifiedMeta(path));
                                } else {
                                    handler.handle(
                                        &event.paths,
                                        AssetSourceEvent::ModifiedAsset(path),
                                    );
                                };
                            }
                            notify::EventKind::Remove(RemoveKind::File) => {
                                let Some((path, is_meta)) = handler.get_path(&event.paths[0])
                                else {
                                    continue;
                                };
                                if is_meta {
                                    handler
                                        .handle(&event.paths, AssetSourceEvent::RemovedMeta(path));
                                } else {
                                    handler
                                        .handle(&event.paths, AssetSourceEvent::RemovedAsset(path));
                                }
                            }
                            notify::EventKind::Remove(RemoveKind::Folder) => {
                                let Some((path, _)) = handler.get_path(&event.paths[0]) else {
                                    continue;
                                };
                                handler.handle(&event.paths, AssetSourceEvent::RemovedFolder(path));
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
    debouncer.watcher().watch(&root, RecursiveMode::Recursive)?;
    debouncer.cache().add_root(&root, RecursiveMode::Recursive);
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
            self.sender.send(event).unwrap();
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
