use libloading::{Library, Symbol};

use bevy_app::{App, CreatePlugin, Plugin};

/// Dynamically links a plugin a the given path. The plugin must export a function with the
/// [`CreatePlugin`] signature named `_bevy_create_plugin`.
///
/// # Safety
///
/// The specified plugin must be linked against the exact same libbevy.so as this program.
/// In addition the `_bevy_create_plugin` symbol must not be manually created, but instead created
/// by deriving `DynamicPlugin` on a unit struct implementing [`Plugin`].
pub unsafe fn dynamically_load_plugin(path: &str) -> (Library, Box<dyn Plugin>) {
    let lib = Library::new(path).unwrap();
    let func: Symbol<CreatePlugin> = lib.get(b"_bevy_create_plugin").unwrap();
    let plugin = Box::from_raw(func());
    (lib, plugin)
}

pub trait DynamicPluginExt {
    /// # Safety
    ///
    /// Same as [`dynamically_load_plugin`].
    unsafe fn load_plugin(&mut self, path: &str) -> &mut Self;
}

impl DynamicPluginExt for App {
    unsafe fn load_plugin(&mut self, path: &str) -> &mut Self {
        let (lib, plugin) = dynamically_load_plugin(path);
        std::mem::forget(lib); // Ensure that the library is not automatically unloaded
        plugin.build(self);
        self
    }
}
