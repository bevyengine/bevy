use bevy_utils::{default, HashMap, HashSet};
use crossbeam_channel::Receiver;
use notify::{Event, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::path::{Path, PathBuf};

/// Watches for changes to files on the local filesystem.
///
/// When hot-reloading is enabled, the [`AssetServer`](crate::AssetServer) uses this to reload
/// assets when their source files are modified.
pub struct FilesystemWatcher {
    pub watcher: RecommendedWatcher,
    pub receiver: Receiver<Result<Event>>,
    pub path_map: HashMap<PathBuf, HashSet<PathBuf>>,
}

impl Default for FilesystemWatcher {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let watcher: RecommendedWatcher = RecommendedWatcher::new(
            move |res| {
                sender.send(res).expect("Watch event send failure.");
            },
            default(),
        )
        .expect("Failed to create filesystem watcher.");
        FilesystemWatcher {
            watcher,
            receiver,
            path_map: default(),
        }
    }
}

impl FilesystemWatcher {
    /// Watch for changes recursively at the provided path.
    pub fn watch<P: AsRef<Path>>(&mut self, to_watch: P, to_reload: PathBuf) -> Result<()> {
        self.path_map
            .entry(to_watch.as_ref().to_owned())
            .or_default()
            .insert(to_reload);
        self.watcher
            .watch(to_watch.as_ref(), RecursiveMode::Recursive)
    }
}
