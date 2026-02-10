//! APIs that return the location of standard user directories.

// Modeled after https://github.com/dirs-dev/dirs-sys-rs/

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
use std::path::PathBuf;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub use windows::preferences_dir;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::preferences_dir;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::preferences_dir;

/// Returns the path to the directory used for application settings. This version
/// always returns `None`.
#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn preferences_dir() -> Option<PathBuf> {
    None
}
