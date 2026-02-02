use bevy_log::{debug, error, warn};
use bevy_tasks::IoTaskPool;
use std::{fs, path::PathBuf};

use dirs::preference_dir;

use crate::{
    prefs::PreferencesStore, prefs_file::serialize_table, PreferencesFile, PreferencesFileContent,
};

/// Persistent storage which uses the local filesystem. Preferences will be located in the
/// OS-specific directory for user preferences.
pub struct StoreFs {
    base_path: Option<PathBuf>,
}

impl StoreFs {
    /// Construct a new filesystem preferences store.
    ///
    /// # Arguments
    /// * `app_name` - The name of the application. This is used to uniquely identify the
    ///   preferences directory so as not to confuse it with other applications' preferences.
    ///   To ensure global uniqueness, it is recommended to use a reverse domain name, e.g.
    ///   "com.example.myapp".
    pub(crate) fn new(app_name: &str) -> Self {
        Self {
            base_path: if let Some(base_dir) = preference_dir() {
                let prefs_path = base_dir.join(app_name);
                debug!("Preferences path: {:?}", prefs_path);
                Some(prefs_path)
            } else {
                warn!("Could not find user configuration directories");
                None
            },
        }
    }
}

impl PreferencesStore for StoreFs {
    /// Returns true if preferences path is valid.
    fn is_valid(&self) -> bool {
        self.base_path.is_some()
    }

    fn create(&self) -> PreferencesFile {
        PreferencesFile::new()
    }

    /// Save a [`PreferencesFile`] to disk.
    ///
    /// # Arguments
    /// * `filename` - the name of the file to be saved
    /// * `contents` - the contents of the file
    fn save(&self, filename: &str, contents: &PreferencesFile) {
        if let Some(base_path) = &self.base_path {
            // Recursively create the preferences directory if it doesn't exist.
            let mut dir_builder = fs::DirBuilder::new();
            dir_builder.recursive(true);
            if let Err(e) = dir_builder.create(base_path.clone()) {
                warn!("Could not create preferences directory: {:?}", e);
                return;
            }

            // Save preferences to temp file
            let temp_path = base_path.join(format!("{filename}.toml.new"));
            if let Err(e) = fs::write(&temp_path, serialize_table(&contents.table)) {
                error!("Error saving preferences file: {}", e);
            }

            // Replace old prefs file with new one.
            let file_path = base_path.join(format!("{filename}.toml"));
            if let Err(e) = fs::rename(&temp_path, file_path) {
                warn!("Could not save preferences file: {:?}", e);
            }
        }
    }

    /// Save the contents of a [`PreferencesFile`] to disk in another thread.
    ///
    /// # Arguments
    /// * `filename` - the name of the file to be saved
    /// * `contents` - the contents of the file
    fn save_async(&self, filename: &str, contents: PreferencesFileContent) {
        if let Some(base_path) = &self.base_path {
            IoTaskPool::get().scope(|scope| {
                scope.spawn(async {
                    // Recursively create the preferences directory if it doesn't exist.
                    let mut dir_builder = fs::DirBuilder::new();
                    dir_builder.recursive(true);
                    if let Err(e) = dir_builder.create(base_path.clone()) {
                        warn!("Could not create preferences directory: {:?}", e);
                        return;
                    }

                    // Save preferences to temp file
                    let temp_path = base_path.join(format!("{filename}.toml.new"));
                    if let Err(e) = fs::write(&temp_path, serialize_table(&contents.0)) {
                        error!("Error saving preferences file: {}", e);
                    }

                    // Replace old prefs file with new one.
                    let file_path = base_path.join(format!("{filename}.toml"));
                    if let Err(e) = fs::rename(&temp_path, file_path) {
                        warn!("Could not save preferences file: {:?}", e);
                    }
                });
            });
        }
    }

    /// Deserialize a preferences file from disk. If the file does not exist, `None` will
    /// be returned.
    ///
    /// # Arguments
    /// * `filename` - The name of the preferences file, without the file extension.
    fn load(&mut self, filename: &str) -> Option<PreferencesFile> {
        let Some(base_path) = &self.base_path else {
            return None;
        };

        let file_path = base_path.join(format!("{filename}.toml"));
        decode_toml_file(&file_path).map(PreferencesFile::from_table)
    }
}

/// Load a preferences file from disk in TOML format.
pub(crate) fn decode_toml_file(file: &PathBuf) -> Option<toml::Table> {
    if file.exists() && file.is_file() {
        let prefs_str = match fs::read_to_string(file) {
            Ok(prefs_str) => prefs_str,
            Err(e) => {
                error!("Error reading preferences file: {}", e);
                return None;
            }
        };

        let table_value = match toml::from_str::<toml::Value>(&prefs_str) {
            Ok(table_value) => table_value,
            Err(e) => {
                error!("Error parsing preferences file: {}", e);
                return None;
            }
        };

        match table_value {
            toml::Value::Table(table) => Some(table),
            _ => {
                error!("Preferences file must be a table");
                None
            }
        }
    } else {
        // Preferences file does not exist yet.
        None
    }
}
