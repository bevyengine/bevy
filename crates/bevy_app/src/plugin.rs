use crate::App;
use std::any::Any;

/// A collection of Bevy App logic and configuration
///
/// Plugins configure an [`App`](crate::App). When an [`App`](crate::App) registers
/// a plugin, the plugin's [`Plugin::build`] function is run.
pub trait Plugin: Any + Send + Sync {
    /// Configures an [`App`]
    ///
    /// # Examples
    /// ```
    /// # use bevy_app::App;
    /// # struct MyPlugin;
    /// # struct MyPluginResource;
    /// # fn my_plugin_system(){}
    ///
    /// impl Plugin for MyPlugin{
    ///     fn build(&self, app: &mut App){
    ///         app.add_startup_system(my_plugin_system);
    ///         app.init_resource(MyPluginResource);
    ///     }
    /// }
    /// ```
    fn build(&self, app: &mut App);
    /// Configures a name for the [`Plugin`]. Primarily for debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// Type representing an unsafe function that returns a mutable pointer to a [`Plugin`].
/// Used for dynamically loading plugins. See bevy_dynamic_plugin/src/loader.rs#dynamically_load_plugin
pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;
