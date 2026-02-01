use bevy_ecs::resource::Resource;
use bevy_platform::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use crate::StoreFs;

#[cfg(target_arch = "wasm32")]
use crate::StoreWasm;

pub use crate::{PreferencesFile, PreferencesFileContent};

// TODO: Think about potential Results:
// NoFile
// NoDirectory
// IOError

/// Abstracts the storage location of the preferences files. This could be a directory on disk,
/// a database, or some other respository.
pub trait PreferencesStore {
    /// Returns true if preferences path is valid.
    fn is_valid(&self) -> bool;

    /// Create a new [`PreferencesFile`] instance. This does not actually save the file until
    /// `save` is called.
    fn create(&self) -> PreferencesFile;

    /// Read a [`PreferencesFile`] from the store.
    fn load(&mut self, filename: &str) -> Option<PreferencesFile>;

    /// Save a [`PreferencesFile`] to the store.
    ///
    /// # Arguments
    /// * `filename` - the filename of the [`PreferencesFile`].
    /// * `file` - the contents of the file.
    fn save(&self, filename: &str, file: &PreferencesFile);

    /// Save a [`PreferencesFile`] to the store in another thread.
    ///
    /// # Arguments
    /// * `filename` - the filename of the [`PreferencesFile`].
    /// * `file` - the contents of the file.
    fn save_async(&self, filename: &str, file: PreferencesFileContent);
}

/// Resource which represents the place where preferences files are stored. This can be either
/// a filesystem directory (when working on a desktop platform) or a virtual directory such
/// as web `LocalStorage`.
///
/// You can access individual preferences files using the `.get()` or `.get_mut()` method. These
/// methods load the preferences into memory if they are not already loaded.
#[derive(Resource)]
pub struct Preferences {
    store: Box<dyn PreferencesStore + Send + Sync + 'static>,
    files: HashMap<String, PreferencesFile>,
}

impl Preferences {
    /// Construct a new `Preferences` resource.
    ///
    /// # Arguments
    /// * `app_name` - The name of the application. This is used to uniquely identify the
    ///   preferences directory so as not to confuse it with other applications' preferences.
    ///   To ensure global uniqueness, it is recommended to use a reverse domain name, e.g.
    ///   "com.example.myapp".
    ///
    ///   This is only used on desktop platforms. On web platforms, the name is ignored.
    ///
    pub fn new(app_name: &str) -> Self {
        Self {
            #[cfg(not(target_arch = "wasm32"))]
            store: Box::new(StoreFs::new(app_name)),
            #[cfg(target_arch = "wasm32")]
            store: Box::new(StoreWasm::new(app_name)),
            files: HashMap::default(),
        }
    }

    /// Returns true if preferences path is valid.
    pub fn is_valid(&self) -> bool {
        self.store.is_valid()
    }

    /// Save all changed `PreferenceFile`s to disk
    ///
    /// # Arguments
    /// * `force` - If true, all preferences will be saved, even if they have not changed.
    pub fn save(&self, force: bool) {
        for (filename, file) in self.files.iter() {
            if file.is_changed() || force {
                file.clear_changed();
                self.store.save(filename, file);
            }
        }
    }

    /// Save all changed `PreferenceFile`s to disk, in another thread.
    ///
    /// # Arguments
    /// * `force` - If true, all preferences will be saved, even if they have not changed.
    pub fn save_async(&self, force: bool) {
        for (filename, file) in self.files.iter() {
            if file.is_changed() || force {
                file.clear_changed();
                self.store.save_async(filename, file.content());
            }
        }
    }

    /// Load and cache a [`PreferencesFile`]. If the file is already loaded, it will be returned
    /// immediately. If the file exists but is not loaded, it will be loaded and returned.
    /// If the file does not exist, or the base preference path cannot be determined, `None` will
    /// be returned.
    ///
    /// Once loaded, a [`PreferencesFile`] will remain in memory.
    ///
    /// # Arguments
    /// * `filename` - The name of the preferences file, without the file extension.
    pub fn get<'a>(&'a mut self, filename: &str) -> Option<&'a PreferencesFile> {
        if !self.files.contains_key(filename)
            && let Some(table) = self.store.load(filename)
        {
            self.files.insert(filename.to_owned(), table);
        };

        self.files.get(filename)
    }

    /// Load and cache a [`PreferencesFile`], or create it if it does not exist. If the file is
    /// already loaded, it will be returned immediately. If the file exists but is not loaded, it
    /// will be loaded and returned. If the file does not exist, a new [`PreferencesFile`] will be
    /// created and returned (but not saved). If the base preference path cannot be determined,
    /// `None` will be returned.
    ///
    /// Once loaded, a [`PreferencesFile`] will remain in memory.
    ///
    /// # Arguments
    /// * `filename` - The name of the preferences file, without the file extension.
    pub fn get_mut<'a>(&'a mut self, filename: &str) -> Option<&'a mut PreferencesFile> {
        if !self.files.contains_key(filename) {
            if let Some(table) = self.store.load(filename) {
                self.files.insert(filename.to_owned(), table);
            } else {
                self.files.insert(filename.to_owned(), self.store.create());
            }
        }

        self.files.get_mut(filename)
    }
}
