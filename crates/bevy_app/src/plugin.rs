use crate::AppBuilder;
use std::any::Any;

/// A collection of Bevy App logic and configuration
///
/// Plugins use [AppBuilder] to configure an [App](crate::App). When an [App](crate::App) registers
/// a plugin, the plugin's [Plugin::build] function is run.
pub trait Plugin: Any + Send + Sync {
    fn build(&self, app: &mut AppBuilder);
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

pub type CreatePlugin = unsafe fn() -> *mut dyn Plugin;
