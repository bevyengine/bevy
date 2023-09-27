use libloading::{Library, Symbol};
use std::ffi::OsStr;
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
pub unsafe fn dynamically_load_plugin<P: AsRef<OsStr>>(
    path: P,
) -> Result<(Library, Box<dyn Plugin>), DynamicPluginLoadError> {
    // SAFETY: the caller must uphold the safety contract for `new`.
    let lib = unsafe { Library::new(path) }.map_err(DynamicPluginLoadError::Library)?;
    // SAFETY: the caller must uphold the safety contract for `get`.
    let func: Symbol<CreatePlugin> =
        unsafe { lib.get(b"_bevy_create_plugin") }.map_err(DynamicPluginLoadError::Plugin)?;
    // SAFETY: the caller must uphold the safety contract for `from_raw`.
    let plugin = unsafe { Box::from_raw(func()) };
    Ok((lib, plugin))
}

pub trait DynamicPluginExt {
    /// # Safety
    ///
    /// Same as [`dynamically_load_plugin`].
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;
}

impl DynamicPluginExt for App {
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        // SAFETY: the caller must uphold the safety contract for `dynamically_load_plugin`.
        let (lib, plugin) = unsafe { dynamically_load_plugin(path) }.unwrap();
        std::mem::forget(lib); // Ensure that the library is not automatically unloaded
        plugin.build(self);
        self
    }
}
