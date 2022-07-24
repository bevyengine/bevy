use crate::App;
use std::any::Any;

/// Portable [`App`] configuration.
///
/// Plugins make it easy to export (and re-import) systems, systems sets, and resources.
///
/// After you import a plugin to an [`App`](crate::App) using [`add_plugin`](App::add_plugin), the app will run its [`build`](Plugin::build) function.
pub trait Plugin: Any + Send + Sync {
    /// Applies the stored configuration to the given [`App`].
    fn build(&self, app: &mut App);
    /// Returns the name of this plugin.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// An alias for an unsafe function that returns a pointer to a [`Plugin`].
/// Used for dynamically loading plugins,
/// as shown in [this example][`bevy_dynamic_plugin/src/loader.rs#dynamically_load_plugin`].
pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;
