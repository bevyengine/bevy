use crate::io::{
    file::{get_asset_path, get_base_path, new_asset_event_debouncer, FilesystemEventHandler},
    memory::Dir,
    AssetSourceEvent, AssetWatcher,
};
use bevy_utils::tracing::warn;
use bevy_utils::{Duration, HashMap};
use notify_debouncer_full::{notify::RecommendedWatcher, Debouncer, FileIdMap};
use parking_lot::RwLock;
use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};

/// A watcher for assets stored in the `embedded` asset source. Embedded assets are assets whose
/// bytes have been embedded into the Rust binary using the [`embedded_asset`](crate::embedded_asset) macro.
/// This watcher will watch for changes to the "source files", read the contents of changed files from the file system
/// and overwrite the initial static bytes of the file embedded in the binary with the new dynamically loaded bytes.
pub struct EmbeddedWatcher {
    _watcher: Debouncer<RecommendedWatcher, FileIdMap>,
}

impl EmbeddedWatcher {
    pub fn new(
        dir: Dir,
        root_paths: Arc<RwLock<HashMap<Box<Path>, PathBuf>>>,
        sender: crossbeam_channel::Sender<AssetSourceEvent>,
        debounce_wait_time: Duration,
    ) -> Self {
        let root = get_base_path();
        let handler = EmbeddedEventHandler {
            dir,
            root: root.clone(),
            sender,
            root_paths,
            last_event: None,
        };
        let watcher = new_asset_event_debouncer(root, debounce_wait_time, handler).unwrap();
        Self { _watcher: watcher }
    }
}

impl AssetWatcher for EmbeddedWatcher {}

/// A [`FilesystemEventHandler`] that uses [`EmbeddedAssetRegistry`](crate::io::embedded::EmbeddedAssetRegistry) to hot-reload
/// binary-embedded Rust source files. This will read the contents of changed files from the file system and overwrite
/// the initial static bytes from the file embedded in the binary.
pub(crate) struct EmbeddedEventHandler {
    sender: crossbeam_channel::Sender<AssetSourceEvent>,
    root_paths: Arc<RwLock<HashMap<Box<Path>, PathBuf>>>,
    root: PathBuf,
    dir: Dir,
    last_event: Option<AssetSourceEvent>,
}
impl FilesystemEventHandler for EmbeddedEventHandler {
    fn begin(&mut self) {
        self.last_event = None;
    }

    fn get_path(&self, absolute_path: &Path) -> Option<(PathBuf, bool)> {
        let (local_path, is_meta) = get_asset_path(&self.root, absolute_path);
        let final_path = self.root_paths.read().get(local_path.as_path())?.clone();
        if is_meta {
            warn!("Meta file asset hot-reloading is not supported yet: {final_path:?}");
        }
        Some((final_path, false))
    }

    fn handle(&mut self, absolute_paths: &[PathBuf], event: AssetSourceEvent) {
        if self.last_event.as_ref() != Some(&event) {
            if let AssetSourceEvent::ModifiedAsset(path) = &event {
                if let Ok(file) = File::open(&absolute_paths[0]) {
                    let mut reader = BufReader::new(file);
                    let mut buffer = Vec::new();

                    // Read file into vector.
                    if reader.read_to_end(&mut buffer).is_ok() {
                        self.dir.insert_asset(path, buffer);
                    }
                }
            }
            self.last_event = Some(event.clone());
            self.sender.send(event).unwrap();
        }
    }
}
