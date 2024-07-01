use std::sync::atomic::{AtomicBool, Ordering};
use bevy_app::{App, Plugin};
use bevy_core::TaskPoolPlugin;

pub(crate) static PLUGIN_INIT: InitChecker = InitChecker::new();

pub struct InitChecker(AtomicBool);

impl InitChecker {
    pub const fn new() -> Self {
        Self(AtomicBool::new(false))
    } 
    
    pub fn is_init(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    
    fn set_init(&self) {
        self.0.store(true, Ordering::Release);
    }
}

#[derive(Default)]
pub struct SocketManagerPlugin;

impl Plugin for SocketManagerPlugin {
    fn build(&self, app: &mut App) {
        if app.is_plugin_added::<TaskPoolPlugin>() {
            PLUGIN_INIT.set_init();
        } else {
            app.add_plugins(TaskPoolPlugin::default());
            PLUGIN_INIT.set_init();
        }
    }
}