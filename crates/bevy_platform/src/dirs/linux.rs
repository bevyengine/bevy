use std::{
    env::{self, home_dir},
    path::PathBuf,
};

/// The path if it's absolute or [`None`]. Empty paths are not absolute.
///
/// [XDG Base Directory Specification] requires that the path specified in environment variables must be absolute. If it's not, we should ignore it and fallback to the default path.
///
/// [XDG Base Directory Specification]: https://specifications.freedesktop.org/basedir/latest/
fn is_absolute_path(path: impl Into<PathBuf>) -> Option<PathBuf> {
    let path = path.into();
    if path.is_absolute() {
        Some(path)
    } else {
        None
    }
}

/// Returns the path to the directory used for application settings.
pub fn preferences_dir() -> Option<PathBuf> {
    // default value for XDG_CONFIG_HOME when unset, empty, or invalid is ~/.config/
    env::var_os("XDG_CONFIG_HOME")
        .and_then(is_absolute_path)
        .or_else(|| home_dir().map(|home| home.join(".config")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_not_absolute() {
        // preferences_dir() depends on is_absolute_path() returning None for empty paths, so we test that here.
        assert!(is_absolute_path("").is_none());
    }
}
