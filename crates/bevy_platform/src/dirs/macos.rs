use std::{env::home_dir, path::PathBuf};

/// Returns the path to the directory used for application settings.
pub fn preferences_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join("Library/Preferences"))
}
