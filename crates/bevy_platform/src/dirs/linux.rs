use std::{
    env::{self, home_dir},
    path::PathBuf,
};

/// Returns the path to the directory used for application settings.
pub fn preferences_dir() -> Option<PathBuf> {
    // default value for XDG_CONFIG_HOME when unset, empty, or invalid is ~/.config/
    env::var_os("XDG_CONFIG_HOME")
        .and_then(super::is_absolute_path)
        .or_else(|| home_dir().map(|home| home.join(".config")))
}
