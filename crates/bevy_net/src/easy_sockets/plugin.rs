use std::sync::atomic::{AtomicBool, Ordering};
use bevy_internal::app::App;
use bevy_internal::core::TaskPoolPlugin;
use bevy_internal::prelude::Plugin;

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

pub struct SocketManagerPlugin {}

impl Plugin for SocketManagerPlugin {
    fn build(&self, app: &mut App) {
        PLUGIN_INIT.set_init();
        todo!()
    }
}