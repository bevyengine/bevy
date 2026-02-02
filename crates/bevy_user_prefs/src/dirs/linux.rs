use std::{env::home_dir, path::PathBuf};

pub fn preferences_dir() -> Option<PathBuf> {
    // TODO: Support XDG_CONFIG
    home_dir().map(|home| home.join(".config"))
}
