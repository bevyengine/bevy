use std::{
    mem::{self, ManuallyDrop},
    sync::{
        self,
        atomic::{AtomicBool, Ordering},
        Arc,
    }, ffi::{OsStr, OsString},
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
    /// Additionally, the concerns from [`DynamicPluginLibraries::mark_for_deallocation`] also apply.
    pub unsafe fn into_raw_parts(self) -> (Box<dyn Plugin>, Library) {
        (self.plugin, ManuallyDrop::into_inner(self.lib))
    }

    /// Creates a leaky dynamic plugin from its raw [`Box<dyn Plugin>`] and [`Library`].
    pub fn from_raw_parts(plugin: Box<dyn Plugin>, lib: Library) -> Self {
        let lib = ManuallyDrop::new(lib);
        Self { plugin, lib }
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
/// The library can be marked for deallocation with [`DynamicPluginLibraries::mark_for_deallocation`],
/// or [`App::mark_plugin_for_deallocation`].
pub struct DynamicPlugin {
    leaking: ManuallyDrop<LeakingDynamicPlugin>,
    should_drop: Arc<AtomicBool>,
}

impl DynamicPlugin {
    /// Coverts this dynamic plugin into its raw [`Box<dyn Plugin>`] and [`Library`].
    ///
    /// # Safety
    ///
    /// The general safety concerns from [`Library::new`] apply here.
    /// Additionally, the concerns from [`DynamicPluginLibraries::mark_for_deallocation`] also apply.
    pub fn into_leaking(mut self) -> LeakingDynamicPlugin {
        self.should_drop.store(false, Ordering::Release);
        // SAFETY: value is immediately dropped
        // and not read during the drop impl because `should_drop` is false.
        unsafe { ManuallyDrop::take(&mut self.leaking) }
    }

    /// Creates a leaky dynamic plugin from its raw [`Box<dyn Plugin>`] and [`Library`].
    pub fn from_leaking(leaking: LeakingDynamicPlugin) -> Self {
        // This is sound because the value of `should_drop` can only be changed with unsafe code.
        Self {
            leaking: ManuallyDrop::new(leaking),
            should_drop: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn should_drop(&self) -> bool {
        self.should_drop.load(Ordering::Acquire)
    }
}

impl Drop for DynamicPlugin {
    fn drop(&mut self) {
        if self.should_drop() {
            // SAFETY: value is never read after taking because this is a drop impl.
            let leaking = unsafe { ManuallyDrop::take(&mut self.leaking) };
            // SAFETY: library is dropped after plugin
            // and caller of `mark_for_deallocation` ensures the invariants.
            let (plugin, _lib) = unsafe { leaking.into_raw_parts() };
            mem::drop(plugin); // explicitly drop plugin before library.
        }
    }
}

impl Plugin for DynamicPlugin {
    fn name(&self) -> &str {
        self.leaking.plugin.name()
    }

    fn is_unique(&self) -> bool {
        self.leaking.plugin.is_unique()
    }

    fn build(&self, app: &mut App) {
        self.leaking.plugin.build(app);
    }
}

/// Stores all dynamic libraries loaded by [`load_plugin`] so they can be manually deallocated.
///
/// [`load_plugin`]: DynamicPluginExt::load_plugin
#[derive(Resource)]
pub struct DynamicPluginLibraries {
    should_drop: HashMap<OsString, sync::Weak<AtomicBool>>,
}

impl DynamicPluginLibraries {
    /// Explcitly marks a dynamically-loaded library for deallocation.
    ///
    /// The dynamic library would otherwise be left leaking until the program is terminated.
    /// The library will be deallocated when its associated plugin is.
    ///
    /// # Safety
    ///
    /// The caller must ensure that all function pointers pointing to code inside the dynamic library
    /// are dropped before the library is unloaded.
    /// This includes all `dyn Trait`, `impl Trait` and `fn(...)` definitions and and types containing these.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use bevy_dynamic_plugin::{DynamicPluginLibraries, DynamicPluginExt};
    /// use bevy_app::App;
    ///
    /// const LIB_NAME: &str = "./libmy_dyn_plugin.so";
    ///
    /// let mut app = App::new();
    /// unsafe { app.load_plugin(LIB_NAME) };
    ///
    /// let mut libs = app.world.remove_resource::<DynamicPluginLibraries>().unwrap();
    /// unsafe { libs.mark_for_deallocation(LIB_NAME) };
    /// ```
    pub unsafe fn mark_for_deallocation<P: AsRef<OsStr>>(&mut self, name: P) {
        let name = &name.as_ref().to_owned();
        if let Some(weak) = self.should_drop.remove(name) {
            if let Some(atomic) = weak.upgrade() {
                atomic.store(true, Ordering::Release);
            }
        }
    }
}

pub trait DynamicPluginExt {
    /// Dynamically links and builds a plugin at the given path.
    ///
    /// The dynamic library is never dropped
    /// and exists in memory until the program is terminated.
    ///
    /// # Safety
    ///
    /// Same as [`dynamically_load_plugin`].
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;

    /// Explcitly deallocates a dynamically-loaded library.
    ///
    /// The dynamic library would otherwise be left leaking until the program is terminated.
    ///
    /// # Safety
    ///
    /// Same as [`DynamicPluginLibraries::mark_for_deallocation`].
    unsafe fn mark_plugin_for_deallocation<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self;
}

impl DynamicPluginExt for App {
    unsafe fn load_plugin<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        let path = path.as_ref();
        let plugin = dynamically_load_plugin(path).unwrap();
        let plugin = DynamicPlugin::from_leaking(plugin);

        let mut libs = self
            .world
            .get_resource_or_insert_with(|| DynamicPluginLibraries {
                should_drop: HashMap::new(),
            });

        libs.should_drop
            .entry(path.to_owned())
            .or_insert_with(|| Arc::downgrade(&plugin.should_drop));

        plugin.build(self);
        self
    }

    unsafe fn mark_plugin_for_deallocation<P: AsRef<OsStr>>(&mut self, path: P) -> &mut Self {
        if let Some(mut libs) = self.world.get_resource_mut::<DynamicPluginLibraries>() {
            libs.mark_for_deallocation(path);
        }
        self
    }
}
