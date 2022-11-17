use libloading::{Library, Symbol};
use thiserror::Error;

use bevy_app::{App, CreatePlugin, Plugin};

/// Errors that can occur when loading a dynamic plugin
#[derive(Debug, Error)]
pub enum DynamicPluginLoadError {
    #[error("cannot load library for dynamic plugin: {0}")]
    Library(libloading::Error),
    #[error("dynamic library does not contain a valid Bevy dynamic plugin")]
    Plugin(libloading::Error),
}

/// Dynamically links a plugin at the given path. The plugin must export a function with the
/// [`CreatePlugin`] signature named `_bevy_create_plugin`.
///
/// # Safety
///
/// The specified plugin must be linked against the exact same libbevy.so as this program.
/// In addition the `_bevy_create_plugin` symbol must not be manually created, but instead created
/// by deriving `DynamicPlugin` on a unit struct implementing [`Plugin`].
pub unsafe fn dynamically_load_plugin(
    path: &str,
) -> Result<(Library, Box<dyn Plugin>), DynamicPluginLoadError> {
    let lib = Library::new(path).map_err(DynamicPluginLoadError::Library)?;
    let func: Symbol<CreatePlugin> = lib
        .get(b"_bevy_create_plugin")
        .map_err(DynamicPluginLoadError::Plugin)?;
    let plugin = Box::from_raw(func());
    Ok((lib, plugin))
}

pub trait DynamicPluginExt {
    /// # Safety
    ///
    /// Same as [`dynamically_load_plugin`].
    unsafe fn load_plugin(&mut self, path: &str) -> &mut Self;
}

impl DynamicPluginExt for App {
    unsafe fn load_plugin(&mut self, path: &str) -> &mut Self {
        let (lib, plugin) = dynamically_load_plugin(path).unwrap();
        std::mem::forget(lib); // Ensure that the library is not automatically unloaded
        plugin.build(self);
        self
    }
}
