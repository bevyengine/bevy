//! Loads a dynamic plugin from the shared library `dynamic_plugin`

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_dynamic_plugin::dynamically_load_plugin;
use libloading::Library;
use parking_lot::Mutex;

/// If the library wasn't stored here, libloading would unload it as soon as we were done with it below.
/// This would cause the program to crash/segfault later when bevy tries to invoke the plugin's systems.
/// Therefore, libraries should be stored such that they outlive the [`App`].
/// A simple method is to use a global mutex.
static LIBRARY: Mutex<Option<Library>> = Mutex::new(None);

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins);

    #[cfg(any(target_family = "windows", target_family = "unix"))]
    {
        // The ending can be the default library ending of the operating system (e.g. .dll, .so).
        let plugin_name = PathBuf::from("dynamic_plugin");

        let (library, plugin) = unsafe { dynamically_load_plugin(plugin_name) }.unwrap();
        app.add_plugins(plugin);
        info!("Loaded plugin!");
        // Make sure the plugin stays alive by storing it in a global variable:
        *LIBRARY.lock() = Some(library);
    }

    app.run();
}
