//! APIs that return the location of standard user directories.

// Modeled after https://github.com/dirs-dev/dirs-sys-rs/

use std::path::PathBuf;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows::preferences_dir as platform_preferences_dir;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos::preferences_dir as platform_preferences_dir;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux::preferences_dir as platform_preferences_dir;

/// Returns the path to the directory used for application settings. This version
/// always returns `None`.
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn preferences_dir() -> Option<PathBuf> {
    None
}

/// Returns the path to the directory used for application settings.
#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
pub fn preferences_dir() -> Option<PathBuf> {
    std::env::var_os("BEVY_SETTINGS_DIR")
        .and_then(is_absolute_path)
        .or_else(platform_preferences_dir)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn empty_is_not_absolute() {
        // preferences_dir() depends on is_absolute_path() returning None for empty paths, so we test that here.
        assert!(is_absolute_path("").is_none());
    }

    #[test]
    #[cfg(target_os = "windows")]
    #[allow(
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason,
        clippy::undocumented_unsafe_blocks,
        unsafe_code
    )]
    fn env_override_windows() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("BEVY_SETTINGS_DIR", r"C:\\Users\data") };
        assert_eq!(preferences_dir(), Some(PathBuf::from(r"C:\\Users\data")));
        unsafe { std::env::remove_var("BEVY_SETTINGS_DIR") };
    }

    #[test]
    #[cfg(target_os = "windows")]
    #[allow(
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason,
        clippy::undocumented_unsafe_blocks,
        unsafe_code
    )]
    fn env_override_windows_relative() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("BEVY_SETTINGS_DIR", r".\Users\data") };
        assert_eq!(preferences_dir(), platform_preferences_dir());
        unsafe { std::env::remove_var("BEVY_SETTINGS_DIR") };
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[allow(
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason,
        clippy::undocumented_unsafe_blocks,
        unsafe_code
    )]
    fn env_override_unix() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("BEVY_SETTINGS_DIR", "/tmp/my_test_settings") };
        assert_eq!(
            preferences_dir(),
            Some(PathBuf::from("/tmp/my_test_settings"))
        );
        unsafe { std::env::remove_var("BEVY_SETTINGS_DIR") };
    }

    #[test]
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[allow(
        clippy::allow_attributes,
        clippy::allow_attributes_without_reason,
        clippy::undocumented_unsafe_blocks,
        unsafe_code
    )]
    fn env_override_unix_relative() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::set_var("BEVY_SETTINGS_DIR", "relative/path") };
        assert_eq!(preferences_dir(), platform_preferences_dir());
        unsafe { std::env::remove_var("BEVY_SETTINGS_DIR") };
    }
}
