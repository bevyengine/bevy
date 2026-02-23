use std::{env::home_dir, path::PathBuf};

/// Returns the path to the directory used for application settings.
pub fn preferences_dir() -> Option<PathBuf> {
    // TODO: Support XDG_CONFIG
    home_dir().map(|home| home.join(".config"))
}
