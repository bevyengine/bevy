use bevy_utils::synccell::SyncCell;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::path::Path;
use std::sync::mpsc::Receiver;

/// Watches for changes to files on the local filesystem.
///
/// When hot-reloading is enabled, the [`AssetServer`](crate::AssetServer) uses this to reload
/// assets when their source files are modified.
pub struct FilesystemWatcher {
    pub watcher: RecommendedWatcher,
    pub receiver: SyncCell<Receiver<Result<Event>>>,
}

impl Default for FilesystemWatcher {
    fn default() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        let watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res| {
                sender.send(res).expect("Watch event send failure.");
            },
            Config::default(),
        )
        .expect("Failed to create filesystem watcher.");
        FilesystemWatcher {
            watcher,
            receiver: SyncCell::new(receiver),
        }
    }
}

impl FilesystemWatcher {
    /// Watch for changes recursively at the provided path.
    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher.watch(path.as_ref(), RecursiveMode::Recursive)
    }
}
