use crate::AppBuilder;
use libloading::{Library, Symbol};
use std::any::Any;

pub trait Plugin: Any + Send + Sync {
    fn build(&self, app: &mut AppBuilder);
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;

pub fn load_plugin(path: &str) -> (Library, Box<dyn Plugin>) {
    let lib = Library::new(path).unwrap();

    unsafe {
        let func: Symbol<CreatePlugin> = lib.get(b"_create_plugin").unwrap();
        let plugin = Box::from_raw(func());
        (lib, plugin)
    }
}
