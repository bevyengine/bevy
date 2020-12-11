use crate::AppBuilder;
use std::any::Any;

/// A collection of Bevy App logic and configuration
///
/// Plugins use [AppBuilder] to configure an [App](crate::App). When an [App](crate::App) registers a plugin, the plugin's [Plugin::build] function is run.
pub trait Plugin: Any + Send + Sync {
    fn build(self, app: &mut AppBuilder);
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }
}

/// An automaticly implemented trait that allows a plugin to be usable when
/// stored in a type erased box.
///
/// This is necessary because the `Plugin::build` method consumes the plugin,
/// and therefore cannot be called from `&dyn Plugin`.
pub trait BoxablePlugin {
    fn name(&self) -> &str;
    fn unbox_and_build(self: Box<Self>, app: &mut AppBuilder);
}

impl<T: Plugin> BoxablePlugin for T {
    fn name(&self) -> &str {
        self.name()
    }
    fn unbox_and_build(self: Box<Self>, app: &mut AppBuilder) {
        (*self).build(app);
    }
}

/// A plugin stored in a box.
///
/// Since `Box<dyn Plugin>` cannot be consumed this type is a reminder of the
/// correct way to store a plugin in a box.
///
/// ## Example
///
/// ```ignore
/// struct CustomPlugin;
///
/// impl Plugin for CustomPlugin {...}
///
/// let b : BoxedPlugin::new (CustomPlugin)
///
/// b.unbox_and_build (app);
/// ```
pub type BoxedPlugin = Box<dyn BoxablePlugin>;

pub type CreatePlugin = unsafe fn() -> *mut dyn BoxablePlugin;
