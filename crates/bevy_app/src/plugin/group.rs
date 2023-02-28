use crate::Plugin;

use bevy_utils::{prelude::default, tracing::warn, HashMap};

use std::any::TypeId;

/// A group of [plugins](`Plugin`).
///
/// These can all be added to an [`App`] in one go with [`add_plugins`](`App::add_plugins`).
pub trait PluginGroup: Sized {
    /// Returns a [`PluginGroupBuilder`].
    fn build(self) -> PluginGroupBuilder;

    /// Sets the value of the given [`Plugin`], if it exists.
    fn set<T: Plugin>(self, plugin: T) -> PluginGroupBuilder {
        self.build().set(plugin)
    }

    /// Returns the name of the [`PluginGroup`].
    ///
    /// This is primarily used for debugging.
    fn name() -> String {
        std::any::type_name::<Self>().to_string()
    }
}

pub(crate) struct PluginGroupEntry {
    pub(crate) plugin: Box<dyn Plugin>,
    pub(crate) enabled: bool,
}

impl PluginGroup for PluginGroupBuilder {
    fn build(self) -> PluginGroupBuilder {
        self
    }
}

/// Facilitates the creation of a [`PluginGroup`].
///
/// Each [`Plugin`] within the group can be [disabled](Self::disable) or [(re)enabled](Self::enable).
pub struct PluginGroupBuilder {
    pub(crate) group_name: String,
    pub(crate) plugins: HashMap<TypeId, PluginGroupEntry>,
}

impl PluginGroupBuilder {
    /// Constructs a new [`PluginGroupBuilder`] with the name of the [`PluginGroup`].
    pub fn new<G: PluginGroup>() -> Self {
        Self {
            group_name: G::name(),
            plugins: default(),
        }
    }

    /// Adds the [`Plugin`] to the [`PluginGroup`]. If the plugin already exists,
    /// its value will be replaced.
    ///
    /// **NOTE:** By default, plugins are enabled when added.
    // This is not confusing, clippy!
    #[allow(clippy::should_implement_trait)]
    pub fn add<T: Plugin>(mut self, plugin: T) -> Self {
        if let Some(entry) = self.plugins.insert(
            TypeId::of::<T>(),
            PluginGroupEntry {
                plugin: Box::new(plugin),
                enabled: true,
            },
        ) {
            if entry.enabled {
                warn!(
                    "Replacing plugin '{}'. Note that this plugin was enabled.",
                    entry.plugin.name()
                )
            }
        }

        self
    }

    /// Replaces the [`Plugin`].
    ///
    /// # Panics
    ///
    /// Panics if the [`Plugin`] does not exist.
    pub fn set<T: Plugin>(mut self, plugin: T) -> Self {
        let entry = self.plugins.get_mut(&TypeId::of::<T>()).unwrap_or_else(|| {
            panic!(
                "'{}' does not exist in this PluginGroup",
                std::any::type_name::<T>(),
            )
        });
        entry.plugin = Box::new(plugin);
        self
    }

    /// Enables the [`Plugin`].
    ///
    /// A [`Plugin`] must be enabled to be built by the [`App`].
    ///
    /// **NOTE:** By default, plugins are enabled when added. If you [`disable`](Self::disable)
    /// a [`Plugin`], you can [`enable`](Self::enable) it again.
    ///
    /// # Panics
    ///
    /// Panics if the [`Plugin`] does not exist.
    pub fn enable<T: Plugin>(mut self) -> Self {
        let mut entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot enable a plugin that does not exist.");
        entry.enabled = true;
        self
    }

    /// Disables the [`Plugin`].
    ///
    /// A disabled [`Plugin`] will not be built by the [`App`] with the rest of the [`PluginGroup`].
    ///
    /// **NOTE:** By default, plugins are enabled when added. If you [`disable`](Self::disable)
    /// a [`Plugin`], you can [`enable`](Self::enable) it again.
    ///
    /// # Panics
    ///
    /// Panics if the [`Plugin`] does not exist.
    pub fn disable<T: Plugin>(mut self) -> Self {
        let mut entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot disable a plugin that does not exist.");
        entry.enabled = false;
        self
    }
}

/// A plugin group which doesn't do anything. Useful for examples:
/// ```rust
/// # use bevy_app::prelude::*;
/// use bevy_app::NoopPluginGroup as MinimalPlugins;
///
/// fn main(){
///     App::new().add_plugins(MinimalPlugins).run();
/// }
/// ```
#[doc(hidden)]
pub struct NoopPluginGroup;

impl PluginGroup for NoopPluginGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::new::<Self>()
    }
}
