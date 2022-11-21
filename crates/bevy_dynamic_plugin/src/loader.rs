use std::mem::ManuallyDrop;

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
) -> Result<LeakingDynamicPlugin, DynamicPluginLoadError> {
    let lib = Library::new(path).map_err(DynamicPluginLoadError::Library)?;
    let func: Symbol<CreatePlugin> = lib
        .get(b"_bevy_create_plugin")
        .map_err(DynamicPluginLoadError::Plugin)?;
    let plugin = Box::from_raw(func());
    Ok(LeakingDynamicPlugin::from_raw_parts(plugin, lib))
}

/// Wraps a dynamically loaded plugin and its associated library so that they can be dropped correctly.
///
/// This struct leaks the library and so its memory is not freed until the program is terminated.
/// This is in contrast to [`DroppingDynamicPlugin`] which automatically drops the library.
pub struct LeakingDynamicPlugin {
    plugin: Box<dyn Plugin>,
    lib: ManuallyDrop<Library>,
}

impl LeakingDynamicPlugin {
    /// Coverts this dynamic plugin into its raw [`Box<dyn Plugin>`] and [`Library`].
    ///
    /// # Safety
    ///
    /// The general safety concerns from [`Library::new`] apply here.
    /// In addition the concerns from [`DroppingDynamicPlugin::from_leaky`] also apply.
    pub unsafe fn into_raw_parts(self) -> (Box<dyn Plugin>, Library) {
        (self.plugin, ManuallyDrop::into_inner(self.lib))
    }

    /// Creates a leaky dynamic plugin from its raw [`Box<dyn Plugin>`] and [`Library`].
    pub fn from_raw_parts(plugin: Box<dyn Plugin>, lib: Library) -> Self {
        Self {
            plugin,
            lib: ManuallyDrop::new(lib),
        }
    }
}

impl Plugin for LeakingDynamicPlugin {
    fn name(&self) -> &str {
        self.plugin.name()
    }

    fn is_unique(&self) -> bool {
        self.plugin.is_unique()
    }

    fn build(&self, app: &mut App) {
        self.plugin.build(app);
    }
}

/// Wraps a dynamically loaded plugin and its associated library so that they can be dropped correctly.
///
/// This struct does not leak the library and so its memory is freed when the app is dropped.
/// This is in contrast to [`LeakingDynamicPlugin`] which leaks the library.
///
/// Leaving dangling function pointers when the app (and therefore library) is dropped is UB.
/// See [`DroppingDynamicPlugin::from_leaky`] for more info.
pub struct DroppingDynamicPlugin {
    plugin: Box<dyn Plugin>,
    lib: Library,
}

impl DroppingDynamicPlugin {
    /// Returns a [`LeakingDynamicPlugin`].
    pub fn into_leaky(self) -> LeakingDynamicPlugin {
        LeakingDynamicPlugin {
            plugin: self.plugin,
            lib: ManuallyDrop::new(self.lib),
        }
    }

    /// Creates an automatically-dropping dynamic plugin from a [`LeakingDynamicPlugin`].
    ///
    /// # Safety
    ///
    /// The caller must ensure that all function pointers pointing to code inside the dynamic library
    /// are dropped before the [`DroppingDynamicPlugin`] is.
    /// This includes all `dyn Trait`, `impl Trait` and `fn(...)` definitions.
    pub unsafe fn from_leaky(leaky: LeakingDynamicPlugin) -> Self {
        Self {
            plugin: leaky.plugin,
            lib: ManuallyDrop::into_inner(leaky.lib),
        }
    }
}

impl Plugin for DroppingDynamicPlugin {
    fn name(&self) -> &str {
        self.plugin.name()
    }

    fn is_unique(&self) -> bool {
        self.plugin.is_unique()
    }

    fn build(&self, app: &mut App) {
        self.plugin.build(app);
    }
}

pub trait DynamicPluginExt {
    /// Dynamically links and builds a plugin at the given path.
    ///
    /// The dynamic library is never dropped
    /// and exists in memory until the program is terminated.
    /// Use [`load_dropping_plugin`] if you need to free this memory.
    ///
    /// # Safety
    ///
    /// Same as [`dynamically_load_plugin`].
    ///
    /// [`load_allocated_plugin`]: `DyanmicPluginExt::load_allocated_plugin`
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;
    /// Dynamically links and builds a plugin at the given path.
    ///
    /// The dynamic library is dropped when the app is,
    /// freeing its allocated memory,
    /// so this method has addition safety concerns compared to [`load_plugin`]
    ///
    /// # Safety
    ///
    /// All the safety invariants from [`dynamically_load_plugin`]
    /// and [`DroppingDynamicPlugin::from_leaky`] must hold true.
    ///
    /// [`load_plugin`]: `DynamicPluginExt::load_plugin`
    unsafe fn load_dropping_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;
}

impl DynamicPluginExt for App {
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        let plugin = dynamically_load_plugin(path).unwrap();
        plugin.build(self);
        self
    }

    unsafe fn load_dropping_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        let leaky = dynamically_load_plugin(path).unwrap();
        // SAFETY: The caller ensures the invariants.
        let plugin = DroppingDynamicPlugin::from_leaky(leaky);
        plugin.build(self);
        self
    }
}
