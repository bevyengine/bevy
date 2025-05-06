//! This crate provides a platform-agnostic interface for accessing the clipboard

#[cfg(any(windows, unix))]
mod desktop;

#[cfg(not(any(windows, unix)))]
mod dummy;

#[cfg(any(windows, unix))]
pub use desktop::*;

#[cfg(not(any(windows, unix)))]
pub use dummy::*;

/// Clipboard plugin
#[derive(Default)]
pub struct ClipboardPlugin;

impl bevy_app::Plugin for ClipboardPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<Clipboard>();
    }
}
