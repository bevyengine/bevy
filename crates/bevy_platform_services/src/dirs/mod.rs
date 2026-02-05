// Modeled after https://github.com/dirs-dev/dirs-sys-rs/

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

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
pub fn preferences_dir() -> Option<PathBuf> {
    None
}
