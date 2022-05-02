use crate::{App, Plugin};
use bevy_utils::{tracing::debug, HashMap};
use std::any::TypeId;

/// Combines multiple [`Plugin`]s into a single unit.
pub trait PluginGroup {
    /// Configures the [`Plugin`]s that are to be added.
    fn build(&mut self, group: &mut PluginGroupBuilder);
}

struct PluginEntry {
    plugin: Box<dyn Plugin>,
    enabled: bool,
}

/// Facilitates the creation and configuration of a [`PluginGroup`].
/// Provides a build ordering to ensure that [`Plugin`]s which produce/require a [`Resource`](bevy_ecs::system::Resource)
/// are built before/after dependent/depending [`Plugin`]s.
#[derive(Default)]
pub struct PluginGroupBuilder {
    plugins: HashMap<TypeId, PluginEntry>,
    order: Vec<TypeId>,
}

impl PluginGroupBuilder {
    /// Appends a [`Plugin`] to the [`PluginGroupBuilder`].
    pub fn add<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        self.order.push(TypeId::of::<T>());
        self.plugins.insert(
            TypeId::of::<T>(),
            PluginEntry {
                plugin: Box::new(plugin),
                enabled: true,
            },
        );
        self
    }

    /// Configures a [`Plugin`] to be built before another plugin.
    pub fn add_before<Target: Plugin, T: Plugin>(&mut self, plugin: T) -> &mut Self {
        let target_index = self
            .order
            .iter()
            .enumerate()
            .find(|(_i, ty)| **ty == TypeId::of::<Target>())
            .map(|(i, _)| i)
            .unwrap_or_else(|| {
                panic!(
                    "Plugin does not exist: {}.",
                    std::any::type_name::<Target>()
                )
            });
        self.order.insert(target_index, TypeId::of::<T>());
        self.plugins.insert(
            TypeId::of::<T>(),
            PluginEntry {
                plugin: Box::new(plugin),
                enabled: true,
            },
        );
        self
    }

    /// Configures a [`Plugin`] to be built after another plugin.
    pub fn add_after<Target: Plugin, T: Plugin>(&mut self, plugin: T) -> &mut Self {
        let target_index = self
            .order
            .iter()
            .enumerate()
            .find(|(_i, ty)| **ty == TypeId::of::<Target>())
            .map(|(i, _)| i)
            .unwrap_or_else(|| {
                panic!(
                    "Plugin does not exist: {}.",
                    std::any::type_name::<Target>()
                )
            });
        self.order.insert(target_index + 1, TypeId::of::<T>());
        self.plugins.insert(
            TypeId::of::<T>(),
            PluginEntry {
                plugin: Box::new(plugin),
                enabled: true,
            },
        );
        self
    }

    /// Enables a [`Plugin`].
    ///
    /// [`Plugin`]s within a [`PluginGroup`] are enabled by default. This function is used to
    /// opt back in to a [`Plugin`] after [disabling](Self::disable) it.
    pub fn enable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot enable a plugin that does not exist.");
        plugin_entry.enabled = true;
        self
    }

    /// Disables a [`Plugin`], preventing it from being added to the [`App`] with the rest of the [`PluginGroup`].
    pub fn disable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot disable a plugin that does not exist.");
        plugin_entry.enabled = false;
        self
    }

    /// Consumes the [`PluginGroupBuilder`] and [builds](Plugin::build) the contained [`Plugin`]s.
    pub fn finish(self, app: &mut App) {
        for ty in self.order.iter() {
            if let Some(entry) = self.plugins.get(ty) {
                if entry.enabled {
                    debug!("added plugin: {}", entry.plugin.name());
                    entry.plugin.build(app);
                }
            }
        }
    }
}
