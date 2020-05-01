use super::AppBuilder;
use libloading::{Library, Symbol};
use std::any::Any;

pub trait AppPlugin: Any + Send + Sync {
    fn build(&self, app: &mut AppBuilder);
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

pub type CreateAppPlugin = unsafe fn() -> *mut dyn AppPlugin;

pub fn load_plugin(path: &str) -> (Library, Box<dyn AppPlugin>) {
    let lib = Library::new(path).unwrap();

    unsafe {
        let func: Symbol<CreateAppPlugin> = lib.get(b"_create_plugin").unwrap();
        let plugin = Box::from_raw(func());
        (lib, plugin)
    }
}
