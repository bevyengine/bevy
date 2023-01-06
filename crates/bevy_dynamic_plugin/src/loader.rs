use std::{
    mem::{self, ManuallyDrop},
    sync::{
        self,
        atomic::{AtomicBool, Ordering},
        Arc,
    }, ffi::{OsStr, OsString},
    ops::DerefMut,
};

use libloading::{Library, Symbol};
use thiserror::Error;

use bevy_app::{App, CreatePlugin, Plugin};
use bevy_ecs::system::Resource;
use bevy_utils::HashMap;

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
) -> Result<DynamicPlugin, DynamicPluginLoadError> {
    let lib = Library::new(path).map_err(DynamicPluginLoadError::Library)?;
    let func: Symbol<CreatePlugin> = lib
        .get(b"_bevy_create_plugin")
        .map_err(DynamicPluginLoadError::Plugin)?;
    let plugin = Box::from_raw(func());
    Ok(DynamicPlugin::from_raw_parts(plugin, lib))
}

/// Wraps an `Arc<Library>` so that the strong count may only be decremented
/// (and the [`Library`] only be dropped) with unsafe code.
///
/// This wrapper does not provide read-only or mutable access to the contained [`Library`].
struct DynamicLibraryAllocation(ManuallyDrop<Arc<Library>>);

impl DynamicLibraryAllocation {
    fn new(lib: Arc<Library>) -> Self {
        DynamicLibraryAllocation(ManuallyDrop::new(lib))
    }

    unsafe fn drop(self) {
        let _ = ManuallyDrop::into_inner(self.0);
        // The `Arc` is dropped here, decrementing the strong count within the `Arc<Library>`
        // and allowing it to be dropped.
    }
}

/// Wraps a dynamically-loaded plugin and its associated library so that they can be dropped correctly.
///
/// The library can be unloaded with [`DynamicPluginLibraries::mark_for_unloading`],
/// or [`App::mark_plugin_for_unloading`].
pub struct DynamicPlugin {
    plugin: Box<dyn Plugin>,
    lib: Arc<Library>,
    dummy_allocation: Option<DynamicLibraryAllocation>,
}

impl DynamicPlugin {
    /// Coverts this dynamic plugin into its raw [`Box<dyn Plugin>`]
    /// and an [`Arc<Library>`](Library).
    ///
    /// The raw [`Library`] can be retrieved with [`Arc::try_unwrap`].
    ///
    /// # Safety
    ///
    /// See [`DynamicPluginLibraries::mark_for_unloading`].
    /// Importantly, the library returned *must not* be unloaded (by dropping) before the plugin.
    pub unsafe fn into_raw_parts(mut self) -> (Box<dyn Plugin>, Arc<Library>) {
        // Ensure (if we can) that there is only one reference to the library (`self.lib`).
        if let Some(allocation) = self.dummy_allocation.take() {
            allocation.drop();
        }

        (self.plugin, self.lib)
    }

    /// Creates a dynamic plugin from its raw [`Box<dyn Plugin>`] and [`Library`].
    pub fn from_raw_parts(plugin: Box<dyn Plugin>, lib: Library) -> Self {
        let lib = Arc::new(lib);
        let dummy_allocation = DynamicLibraryAllocation::new(Arc::clone(&lib));
        Self {
            plugin,
            lib,
            dummy_allocation: Some(dummy_allocation),
        }
    }
}

impl Plugin for DynamicPlugin {
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

/// Stores all dynamic libraries loaded by [`load_plugin`] so they can be manually unloaded.
///
/// [`load_plugin`]: DynamicPluginExt::load_plugin
#[derive(Resource)]
pub struct DynamicPluginLibraries {
    libraries: HashMap<OsString, DynamicLibraryAllocation>,
}

impl DynamicPluginLibraries {
    /// Explcitly marks a dynamically-loaded library to be unloaded.
    /// The library is specified by the path used to load it.
    ///
    /// The dynamic library would otherwise be left leaking until the program is terminated.
    /// After calling `mark_for_unloading`, the library will then be dropped and unloaded
    /// when its associated plugin is or when the `DynamicPluginLibraries` resource is dropped,
    /// whichever happens last.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all function pointers pointing to code inside the dynamic library
    /// are dropped before the library is unloaded.
    /// This includes all `dyn Trait`, `impl Trait` and `fn(...)` definitions and and types containing these.
    ///
    /// Additionally, the termination routines in the library can impose arbitrary safety
    /// restrictions on unloading the library. Calling `mark_for_unloading` implies that they are safe.
    ///
    /// # Examples
    ///
    /// Using a system to unload a library. The library is still loaded until the app is dropped.
    ///
    /// ```no_run
    /// use bevy_dynamic_plugin::{DynamicPluginLibraries, DynamicPluginExt};
    /// use bevy_app::App;
    ///
    /// const LIB_NAME: &str = "./libmy_dyn_plugin.so";
    ///
    /// let mut app = App::new();
    /// unsafe { app.load_plugin(LIB_NAME) };
    /// app.add_system(remove_library);
    ///
    /// fn remove_library(libs: ResMut<DynamicPluginLibraries>) {
    ///     unsafe { libs.mark_for_unloading(LIB_NAME) };
    ///     // Library is still loaded at this point.
    /// }
    /// ```
    pub unsafe fn mark_for_unloading<P: AsRef<OsStr>>(&mut self, name: P) {
        let name = name.as_ref().to_owned();
        if let Some(allocation) = self.libraries.remove(&name) {
            allocation.drop();
        }
    }
}

pub trait DynamicPluginExt {
    /// Dynamically links and builds a plugin at the given path.
    ///
    /// The dynamic library is never dropped
    /// and exists in memory until the program is terminated,
    /// unless ``
    ///
    /// # Safety
    ///
    /// Same as [`dynamically_load_plugin`].
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;

    /// Explcitly marks a dynamically-loaded library to be unloaded.
    /// The library is specified by the path used to load it.
    ///
    /// See [`DynamicPluginLibraries::mark_for_unloading`] for a more detailed explanation.
    ///
    /// # Safety
    ///
    /// See [`DynamicPluginLibraries::mark_for_unloading`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use bevy_dynamic_plugin::{DynamicPluginLibraries, DynamicPluginExt};
    /// use bevy_app::App;
    ///
    /// const LIB_NAME: &str = "./libmy_dyn_plugin.so";
    ///
    /// let mut app = App::new();
    /// unsafe { app.load_plugin(LIB_NAME) };
    ///
    /// unsafe { app.mark_plugin_for_unloading(LIB_NAME) };
    ///
    /// // The library is unloaded here where the app is dropped, not when `mark_plugin_for_unloading` is called.
    /// ```
    unsafe fn mark_plugin_for_unloading<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;
}

impl DynamicPluginExt for App {
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        let path = path.as_ref();
        let mut plugin = dynamically_load_plugin(path).unwrap();

        let mut libs = self
            .world
            .get_resource_or_insert_with(|| DynamicPluginLibraries {
                libraries: HashMap::new(),
            });

        // Move the `DynamicLibraryAllocation` into `DynamicPluginLibraries`
        // so it may be dropped with `mark_for_unloading`.
        libs.libraries
            .entry(path.to_owned())
            .or_insert_with(|| plugin.dummy_allocation.take().unwrap());

        plugin.build(self);
        self
    }

    unsafe fn mark_plugin_for_unloading<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        if let Some(mut libs) = self.world.get_resource_mut::<DynamicPluginLibraries>() {
            libs.mark_for_unloading(path);
        }
        self
    }
}
