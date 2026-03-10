use crate::prefs_file::serialize_table;
use crate::{prefs::PreferencesStore, PreferencesFile, PreferencesFileContent};
use bevy_log::error;
use bevy_tasks::IoTaskPool;
use web_sys::window;

/// Persistent storage which uses browser local storage.
pub(crate) struct PreferencesStore {
    app_name: String,
}

impl PreferencesStore {
    /// Construct a new preferences store for browser local storage.
    ///
    /// # Arguments
    /// * `app_name` - The name of the application. See [`crate::PreferencesPlugin`] for usage.
    pub fn new(app_name: &str) -> Self {
        Self {
            app_name: app_name.to_owned(),
        }
    }

    /// Returns the storage key for a given filename. This consists of the app name combined
    /// with the filename.
    fn storage_key(&self, filename: &str) -> String {
        format!("{}-{}", self.app_name, filename)
    }

    /// Save a [`toml::Table`] to browser storage, synchronously.
    ///
    /// # Arguments
    /// * `filename` - the name of the file to be saved
    /// * `contents` - the contents of the file
    pub(crate) fn save(&self, filename: &str, contents: &PreferencesFile) {
        if let Ok(Some(storage)) = window().unwrap().local_storage() {
            let toml_str = serialize_table(&contents.table);
            storage
                .set_item(&self.storage_key(filename).as_str(), &toml_str)
                .unwrap();
        }
    }

    /// Save the content of a [`toml::Table`] to local storage, in another thread.
    ///
    /// # Arguments
    /// * `filename` - the name of the file to be saved
    /// * `contents` - the contents of the file
    pub(crate) fn save_async(&self, filename: &str, contents: PreferencesFileContent) {
        IoTaskPool::get().scope(|scope| {
            scope.spawn(async {
                if let Ok(Some(storage)) = window().unwrap().local_storage() {
                    let toml_str = serialize_table(&contents.0);
                    storage
                        .set_item(&self.storage_key(filename).as_str(), &toml_str)
                        .unwrap();
                }
            });
        });
    }

    /// Deserialize a [`toml::Table`]. If the file does not exist, `None` will
    /// be returned.
    ///
    /// # Arguments
    /// * `filename` - The name of the preferences file, without the file extension.
    pub(crate) fn load(&mut self, filename: &str) -> Option<PreferencesFile> {
        if let Ok(Some(storage)) = window().unwrap().local_storage() {
            let storage_key = self.storage_key(filename);
            let Ok(Some(toml_str)) = storage.get_item(&storage_key) else {
                return None;
            };

            let table_value = match toml::from_str::<toml::Value>(&toml_str) {
                Ok(table_value) => table_value,
                Err(e) => {
                    error!("Error parsing preferences file: {}", e);
                    return None;
                }
            };

            match table_value {
                toml::Value::Table(table) => Some(PreferencesFile::from_table(table)),
                _ => {
                    error!("Preferences file must be a table");
                    None
                }
            }
        } else {
            None
        }
    }
}
