use crate::AppBuilder;
use libloading::{Library, Symbol};
use std::any::Any;

/// A collection of Bevy App logic and configuration
///
/// Plugins use [AppBuilder] to configure an [App](crate::App). When an [App](crate::App) registers a plugin, the plugin's [Plugin::build] function is run.
pub trait Plugin: Any + Send + Sync {
    fn build(&self, app: &mut AppBuilder);
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;

/// Dynamically links a plugin a the given path. The plugin must export the [CreatePlugin] function.
pub fn dynamically_load_plugin(path: &str) -> (Library, Box<dyn Plugin>) {
    let lib = Library::new(path).unwrap();

    unsafe {
        let func: Symbol<CreatePlugin> = lib.get(b"_create_plugin").unwrap();
        let plugin = Box::from_raw(func());
        (lib, plugin)
    }
}
