use crossbeam_channel::Receiver;
use notify::{Event, RecommendedWatcher, RecursiveMode, Result, Watcher};
use std::{fmt, path::Path};

/// Watches for changes to assets on the filesystem. This is used by the `AssetServer` to reload them
pub struct FilesystemWatcher {
    pub watcher: RecommendedWatcher,
    pub receiver: Receiver<Result<Event>>,
}

impl fmt::Debug for FilesystemWatcher {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct NoDebug;

        impl fmt::Debug for NoDebug {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "Type doesn't implement Debug")
            }
        }

        f.debug_struct("FilesystemWatcher")
            .field("watcher", &NoDebug)
            .field("receiver", &self.receiver)
            .finish()
    }
}

impl Default for FilesystemWatcher {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let watcher: RecommendedWatcher = Watcher::new_immediate(move |res| {
            sender.send(res).expect("Watch event send failure");
        })
        .expect("Failed to create filesystem watcher");
        FilesystemWatcher { watcher, receiver }
    }
}

impl FilesystemWatcher {
    pub fn watch<P: AsRef<Path>>(&mut self, path: P) -> Result<()> {
        self.watcher.watch(path, RecursiveMode::Recursive)
    }
}
