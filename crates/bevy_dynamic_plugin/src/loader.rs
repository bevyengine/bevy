#![allow(unsafe_code)]
#![allow(deprecated)]

use libloading::{Library, Symbol};
use std::ffi::OsStr;
use thiserror::Error;

use bevy_app::{App, CreatePlugin, Plugin};

/// Errors that can occur when loading a dynamic plugin
#[derive(Debug, Error)]
#[deprecated(
    since = "0.14.0",
    note = "The current dynamic plugin system is unsound and will be removed in 0.15."
)]
pub enum DynamicPluginLoadError {
    /// An error occurred when loading a dynamic library.
    #[error("cannot load library for dynamic plugin: {0}")]
    Library(#[source] libloading::Error),
    /// An error occurred when loading a library without a valid Bevy plugin.
    #[error("dynamic library does not contain a valid Bevy dynamic plugin")]
    Plugin(#[source] libloading::Error),
}

/// Dynamically links a plugin at the given path. The plugin must export a function with the
/// [`CreatePlugin`] signature named `_bevy_create_plugin`.
///
/// # Safety
///
/// The specified plugin must be linked against the exact same `libbevy.so` as this program.
/// In addition the `_bevy_create_plugin` symbol must not be manually created, but instead created
/// by deriving `DynamicPlugin` on a unit struct implementing [`Plugin`].
///
/// Dynamically loading plugins is orchestrated through dynamic linking. When linking against
/// foreign code, initialization routines may be run (as well as termination routines when the
/// program exits). The caller of this function is responsible for ensuring these routines are
/// sound. For more information, please see the safety section of [`libloading::Library::new`].
#[deprecated(
    since = "0.14.0",
    note = "The current dynamic plugin system is unsound and will be removed in 0.15."
)]
pub unsafe fn dynamically_load_plugin<P: AsRef<OsStr>>(
    path: P,
) -> Result<(Library, Box<dyn Plugin>), DynamicPluginLoadError> {
    // SAFETY: Caller must follow the safety requirements of Library::new.
    let lib = unsafe { Library::new(path).map_err(DynamicPluginLoadError::Library)? };

    // SAFETY: Loaded plugins are not allowed to specify `_bevy_create_plugin` symbol manually, but
    // must instead automatically generate it through `DynamicPlugin`.
    let func: Symbol<CreatePlugin> = unsafe {
        lib.get(b"_bevy_create_plugin")
            .map_err(DynamicPluginLoadError::Plugin)?
    };

    // SAFETY: `func` is automatically generated and is guaranteed to return a pointer created using
    // `Box::into_raw`.
    let plugin = unsafe { Box::from_raw(func()) };

    Ok((lib, plugin))
}

/// An extension trait for [`App`] that allows loading dynamic plugins.
#[deprecated(
    since = "0.14.0",
    note = "The current dynamic plugin system is unsound and will be removed in 0.15."
)]
pub trait DynamicPluginExt {
    /// Dynamically links a plugin at the given path, registering the plugin.
    ///
    /// For more details, see [`dynamically_load_plugin`].
    ///
    /// # Safety
    ///
    /// See [`dynamically_load_plugin`]'s safety section.
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;
}

impl DynamicPluginExt for App {
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        // SAFETY: Follows the same safety requirements as `dynamically_load_plugin`.
        let (lib, plugin) = unsafe { dynamically_load_plugin(path).unwrap() };
        std::mem::forget(lib); // Ensure that the library is not automatically unloaded
        plugin.build(self);
        self
    }
}
