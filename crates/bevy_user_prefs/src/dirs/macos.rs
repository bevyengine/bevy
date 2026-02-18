use std::{env::home_dir, path::PathBuf};

pub fn preferences_dir() -> Option<PathBuf> {
    home_dir().map(|home| home.join("Library/Preferences"))
}
