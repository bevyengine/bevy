use crate::io::{AssetSourceEvent, AssetWatcher};
use anyhow::Result;
use bevy_log::error;
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

pub struct FileWatcher {
    _watcher: Debouncer<RecommendedWatcher, FileIdMap>,
}

impl FileWatcher {
    pub fn new(
        root: PathBuf,
        sender: Sender<AssetSourceEvent>,
        debounce_wait_time: Duration,
    ) -> Result<Self, notify::Error> {
        let owned_root = root.clone();
        let mut debouncer = new_debouncer(
            debounce_wait_time,
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        for event in events.iter() {
                            match event.kind {
                                notify::EventKind::Create(CreateKind::File) => {
                                    let (path, is_meta) =
                                        get_asset_path(&owned_root, &event.paths[0]);
                                    if is_meta {
                                        sender.send(AssetSourceEvent::AddedMeta(path)).unwrap();
                                    } else {
                                        sender.send(AssetSourceEvent::AddedAsset(path)).unwrap();
                                    }
                                }
                                notify::EventKind::Create(CreateKind::Folder) => {
                                    let (path, _) = get_asset_path(&owned_root, &event.paths[0]);
                                    sender.send(AssetSourceEvent::AddedFolder(path)).unwrap();
                                }
                                notify::EventKind::Access(AccessKind::Close(AccessMode::Write)) => {
                                    let (path, is_meta) =
                                        get_asset_path(&owned_root, &event.paths[0]);
                                    if is_meta {
                                        sender.send(AssetSourceEvent::ModifiedMeta(path)).unwrap();
                                    } else {
                                        sender.send(AssetSourceEvent::ModifiedAsset(path)).unwrap();
                                    }
                                }
                                notify::EventKind::Remove(RemoveKind::Any) |
                                // Because this is debounced over a reasonable period of time, "From" events are assumed to be "dangling" without
                                // a follow up "To" event. Without debouncing, "From" -> "To" -> "Both" events are emitted for renames.
                                // If a From is dangling, it is assumed to be "removed" from the context of the asset system.
                                notify::EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
                                    let (path, is_meta) =
                                        get_asset_path(&owned_root, &event.paths[0]);
                                    sender
                                        .send(AssetSourceEvent::RemovedUnknown { path, is_meta })
                                        .unwrap();
                                }
                                notify::EventKind::Create(CreateKind::Any)
                                | notify::EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
                                    let (path, is_meta) =
                                        get_asset_path(&owned_root, &event.paths[0]);
                                    let event = if event.paths[0].is_dir() {
                                        AssetSourceEvent::AddedFolder(path)
                                    } else if is_meta {
                                        AssetSourceEvent::AddedMeta(path)
                                    } else {
                                        AssetSourceEvent::AddedAsset(path)
                                    };
                                    sender.send(event).unwrap();
                                }
                                notify::EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
                                    let (old_path, old_is_meta) =
                                        get_asset_path(&owned_root, &event.paths[0]);
                                    let (new_path, new_is_meta) =
                                        get_asset_path(&owned_root, &event.paths[1]);
                                    // only the new "real" path is considered a directory
                                    if event.paths[1].is_dir() {
                                        sender
                                            .send(AssetSourceEvent::RenamedFolder {
                                                old: old_path,
                                                new: new_path,
                                            })
                                            .unwrap();
                                    } else {
                                        match (old_is_meta, new_is_meta) {
                                            (true, true) => {
                                                sender
                                                    .send(AssetSourceEvent::RenamedMeta {
                                                        old: old_path,
                                                        new: new_path,
                                                    })
                                                    .unwrap();
                                            }
                                            (false, false) => {
                                                sender
                                                    .send(AssetSourceEvent::RenamedAsset {
                                                        old: old_path,
                                                        new: new_path,
                                                    })
                                                    .unwrap();
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
                                    let (path, is_meta) =
                                        get_asset_path(&owned_root, &event.paths[0]);
                                    if event.paths[0].is_dir() {
                                        // modified folder means nothing in this case
                                    } else if is_meta {
                                        sender.send(AssetSourceEvent::ModifiedMeta(path)).unwrap();
                                    } else {
                                        sender.send(AssetSourceEvent::ModifiedAsset(path)).unwrap();
                                    };
                                }
                                notify::EventKind::Remove(RemoveKind::File) => {
                                    let (path, is_meta) =
                                        get_asset_path(&owned_root, &event.paths[0]);
                                    if is_meta {
                                        sender.send(AssetSourceEvent::RemovedMeta(path)).unwrap();
                                    } else {
                                        sender.send(AssetSourceEvent::RemovedAsset(path)).unwrap();
                                    }
                                }
                                notify::EventKind::Remove(RemoveKind::Folder) => {
                                    let (path, _) = get_asset_path(&owned_root, &event.paths[0]);
                                    sender.send(AssetSourceEvent::RemovedFolder(path)).unwrap();
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
        Ok(Self {
            _watcher: debouncer,
        })
    }
}

impl AssetWatcher for FileWatcher {}

pub(crate) fn get_asset_path(root: &Path, absolute_path: &Path) -> (PathBuf, bool) {
    let relative_path = absolute_path.strip_prefix(root).unwrap();
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
