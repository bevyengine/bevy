mod group;
mod store;
pub use group::*;
pub(crate) use store::*;

use downcast_rs::{impl_downcast, Downcast};

use crate::App;
use std::any::{Any, TypeId};

/// A set of saved [`App`] additions.
///
/// Plugins allow exporting and importing [`App`] functionality. All of Bevy's engine features
/// outside of the underlying ECS are actually implemented and imported as plugins.
///
/// **Note:** Plugins are identified by their [`TypeId`]. A plugin type can only be added
/// to an [`App`] once.
pub trait Plugin: Downcast + Any + Send + Sync {
    /// Runs after all plugins have been added.
    ///
    /// This is where your plugin can add any systems, components, and resources.
    fn build(&self, app: &mut App);

    /// Runs after all plugins have been built, but before the app can be run.
    ///
    /// If other plugins needed access to resource during their build step, but you now want
    /// to remove it (e.g. to send it to another thread), you can do so here.
    fn setup(&self, _app: &mut App) {
        // default implementation is to do nothing
    }

    /// Returns the name of the plugin.
    ///
    /// This is mainly used for debugging.
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Returns the list of plugins that must be built before the plugin.
    fn depends_on(&self) -> Vec<TypeId> {
        Vec::new()
    }

    /// Returns the list of plugins that the plugin can substitute.
    ///
    /// **Note:** The [`App`] won't build the substituted plugins if present, so this plugin
    /// **must** provide all of the resources expected by their dependents.
    fn subs_for(&self) -> Vec<TypeId> {
        Vec::new()
    }
}

impl_downcast!(Plugin);

/// A type representing an unsafe function that returns a mutable pointer to a [`Plugin`].
/// It is used for dynamically loading plugins.
///
/// See `bevy_dynamic_plugin/src/loader.rs#dynamically_load_plugin`.
pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;
