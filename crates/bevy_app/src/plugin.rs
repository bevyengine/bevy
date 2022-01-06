use crate::App;
use std::any::Any;

/// A collection of Bevy App logic and configuration
///
/// Plugins configure an [`App`](crate::App). When an [`App`](crate::App) registers
/// a plugin, the plugin's [`Plugin::build`] function is run.
pub trait Plugin: Any + Send + Sync {
    /// Configures the [`App`] to which this plugin is added.
    fn build(&self, app: &mut App);
    /// Configures a name for the [`Plugin`]. Primarily for debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Type representing an unsafe function that returns a mutable pointer to a [`Plugin`].
/// Used for dynamically loading plugins. See bevy_dynamic_plugin/src/loader.rs#dynamically_load_plugin
pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;
