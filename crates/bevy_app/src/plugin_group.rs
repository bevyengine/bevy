use crate::{App, AppError, Plugin};
use bevy_utils::{tracing::debug, tracing::warn, HashMap};
use std::any::TypeId;

/// Combines multiple [`Plugin`]s into a single unit.
pub trait PluginGroup: Sized {
    /// Configures the [`Plugin`]s that are to be added.
    fn build(self) -> PluginGroupBuilder;
    /// Configures a name for the [`PluginGroup`] which is primarily used for debugging.
    fn name() -> String {
        std::any::type_name::<Self>().to_string()
    }
    /// Sets the value of the given [`Plugin`], if it exists
    fn set<T: Plugin>(self, plugin: T) -> PluginGroupBuilder {
        self.build().set(plugin)
    }
}

struct PluginEntry {
    plugin: Box<dyn Plugin>,
    enabled: bool,
}

impl PluginGroup for PluginGroupBuilder {
    fn build(self) -> PluginGroupBuilder {
        self
    }
}

/// Facilitates the creation and configuration of a [`PluginGroup`].
/// Provides a build ordering to ensure that [`Plugin`]s which produce/require a [`Resource`](bevy_ecs::system::Resource)
/// are built before/after dependent/depending [`Plugin`]s. [`Plugin`]s inside the group
/// can be disabled, enabled or reordered.
pub struct PluginGroupBuilder {
    group_name: String,
    plugins: HashMap<TypeId, PluginEntry>,
    order: Vec<TypeId>,
}

impl PluginGroupBuilder {
    /// Start a new builder for the [`PluginGroup`].
    pub fn start<PG: PluginGroup>() -> Self {
        Self {
            group_name: PG::name(),
            plugins: Default::default(),
            order: Default::default(),
        }
    }

    /// Finds the index of a target [`Plugin`]. Panics if the target's [`TypeId`] is not found.
    fn index_of<Target: Plugin>(&self) -> usize {
        let index = self
            .order
            .iter()
            .position(|&ty| ty == TypeId::of::<Target>());

        match index {
            Some(i) => i,
            None => panic!(
                "Plugin does not exist in group: {}.",
                std::any::type_name::<Target>()
            ),
        }
    }

    // Insert the new plugin as enabled, and removes its previous ordering if it was
    // already present
    fn upsert_plugin_state<T: Plugin>(&mut self, plugin: T, added_at_index: usize) {
        if let Some(entry) = self.plugins.insert(
            TypeId::of::<T>(),
            PluginEntry {
                plugin: Box::new(plugin),
                enabled: true,
            },
        ) {
            if entry.enabled {
                warn!(
                    "You are replacing plugin '{}' that was not disabled.",
                    entry.plugin.name()
                );
            }
            if let Some(to_remove) = self
                .order
                .iter()
                .enumerate()
                .find(|(i, ty)| *i != added_at_index && **ty == TypeId::of::<T>())
                .map(|(i, _)| i)
            {
                self.order.remove(to_remove);
            }
        }
    }

    /// Sets the value of the given [`Plugin`], if it exists.
    ///
    /// # Panics
    ///
    /// Panics if the [`Plugin`] does not exist.
    pub fn set<T: Plugin>(mut self, plugin: T) -> Self {
        let entry = self.plugins.get_mut(&TypeId::of::<T>()).unwrap_or_else(|| {
            panic!(
                "{} does not exist in this PluginGroup",
                std::any::type_name::<T>(),
            )
        });
        entry.plugin = Box::new(plugin);
        self
    }

    /// Adds the plugin [`Plugin`] at the end of this [`PluginGroupBuilder`]. If the plugin was
    /// already in the group, it is removed from its previous place.
    // This is not confusing, clippy!
    #[allow(clippy::should_implement_trait)]
    pub fn add<T: Plugin>(mut self, plugin: T) -> Self {
        let target_index = self.order.len();
        self.order.push(TypeId::of::<T>());
        self.upsert_plugin_state(plugin, target_index);
        self
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] before the plugin of type `Target`.
    /// If the plugin was already the group, it is removed from its previous place. There must
    /// be a plugin of type `Target` in the group or it will panic.
    pub fn add_before<Target: Plugin, T: Plugin>(mut self, plugin: T) -> Self {
        let target_index = self.index_of::<Target>();
        self.order.insert(target_index, TypeId::of::<T>());
        self.upsert_plugin_state(plugin, target_index);
        self
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] after the plugin of type `Target`.
    /// If the plugin was already the group, it is removed from its previous place. There must
    /// be a plugin of type `Target` in the group or it will panic.
    pub fn add_after<Target: Plugin, T: Plugin>(mut self, plugin: T) -> Self {
        let target_index = self.index_of::<Target>() + 1;
        self.order.insert(target_index, TypeId::of::<T>());
        self.upsert_plugin_state(plugin, target_index);
        self
    }

    /// Enables a [`Plugin`].
    ///
    /// [`Plugin`]s within a [`PluginGroup`] are enabled by default. This function is used to
    /// opt back in to a [`Plugin`] after [disabling](Self::disable) it. If there are no plugins
    /// of type `T` in this group, it will panic.
    pub fn enable<T: Plugin>(mut self) -> Self {
        let plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot enable a plugin that does not exist.");
        plugin_entry.enabled = true;
        self
    }

    /// Disables a [`Plugin`], preventing it from being added to the [`App`] with the rest of the
    /// [`PluginGroup`]. The disabled [`Plugin`] keeps its place in the [`PluginGroup`], so it can
    /// still be used for ordering with [`add_before`](Self::add_before) or
    /// [`add_after`](Self::add_after), or it can be [re-enabled](Self::enable). If there are no
    /// plugins of type `T` in this group, it will panic.
    pub fn disable<T: Plugin>(mut self) -> Self {
        let plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot disable a plugin that does not exist.");
        plugin_entry.enabled = false;
        self
    }

    /// Consumes the [`PluginGroupBuilder`] and [builds](Plugin::build) the contained [`Plugin`]s
    /// in the order specified.
    ///
    /// # Panics
    ///
    /// Panics if one of the plugin in the group was already added to the application.
    #[track_caller]
    pub fn finish(mut self, app: &mut App) {
        for ty in &self.order {
            if let Some(entry) = self.plugins.remove(ty) {
                if entry.enabled {
                    debug!("added plugin: {}", entry.plugin.name());
                    if let Err(AppError::DuplicatePlugin { plugin_name }) =
                        app.add_boxed_plugin(entry.plugin)
                    {
                        panic!(
                            "Error adding plugin {} in group {}: plugin was already added in application",
                            plugin_name,
                            self.group_name
                        );
                    }
                }
            }
        }
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
        PluginGroupBuilder::start::<Self>()
    }
}

#[cfg(test)]
mod tests {
    use super::PluginGroupBuilder;
    use crate::{App, NoopPluginGroup, Plugin};

    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, _: &mut App) {}
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, _: &mut App) {}
    }

    struct PluginC;
    impl Plugin for PluginC {
        fn build(&self, _: &mut App) {}
    }

    #[test]
    fn basic_ordering() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC);

        assert_eq!(
            group.order,
            vec![
                std::any::TypeId::of::<PluginA>(),
                std::any::TypeId::of::<PluginB>(),
                std::any::TypeId::of::<PluginC>(),
            ]
        );
    }

    #[test]
    fn add_after() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add_after::<PluginA, PluginC>(PluginC);

        assert_eq!(
            group.order,
            vec![
                std::any::TypeId::of::<PluginA>(),
                std::any::TypeId::of::<PluginC>(),
                std::any::TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn add_before() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add_before::<PluginB, PluginC>(PluginC);

        assert_eq!(
            group.order,
            vec![
                std::any::TypeId::of::<PluginA>(),
                std::any::TypeId::of::<PluginC>(),
                std::any::TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn readd() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC)
            .add(PluginB);

        assert_eq!(
            group.order,
            vec![
                std::any::TypeId::of::<PluginA>(),
                std::any::TypeId::of::<PluginC>(),
                std::any::TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn readd_after() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC)
            .add_after::<PluginA, PluginC>(PluginC);

        assert_eq!(
            group.order,
            vec![
                std::any::TypeId::of::<PluginA>(),
                std::any::TypeId::of::<PluginC>(),
                std::any::TypeId::of::<PluginB>(),
            ]
        );
    }

    #[test]
    fn readd_before() {
        let group = PluginGroupBuilder::start::<NoopPluginGroup>()
            .add(PluginA)
            .add(PluginB)
            .add(PluginC)
            .add_before::<PluginB, PluginC>(PluginC);

        assert_eq!(
            group.order,
            vec![
                std::any::TypeId::of::<PluginA>(),
                std::any::TypeId::of::<PluginC>(),
                std::any::TypeId::of::<PluginB>(),
            ]
        );
    }
}
