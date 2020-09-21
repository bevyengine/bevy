use libloading::{Library, Symbol};

use bevy_app::{AppBuilder, CreatePlugin, Plugin};

/// Dynamically links a plugin a the given path. The plugin must export the [CreatePlugin] function.
pub fn dynamically_load_plugin(path: &str) -> (Library, Box<dyn Plugin>) {
    let lib = Library::new(path).unwrap();

    unsafe {
        let func: Symbol<CreatePlugin> = lib.get(b"_create_plugin").unwrap();
        let plugin = Box::from_raw(func());
        (lib, plugin)
    }
}

pub trait DynamicPluginExt {
    fn load_plugin(&mut self, path: &str) -> &mut Self;
}

impl DynamicPluginExt for AppBuilder {
    fn load_plugin(&mut self, path: &str) -> &mut Self {
        let (_lib, plugin) = dynamically_load_plugin(path);
        log::debug!("loaded plugin: {}", plugin.name());
        plugin.build(self);
        self
    }
}
