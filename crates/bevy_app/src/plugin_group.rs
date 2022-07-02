use crate::{App, Plugin};
use bevy_utils::{tracing::debug, tracing::warn, HashMap};
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
/// are built before/after dependent/depending [`Plugin`]s. [`Plugin`]s inside the group
/// can be disabled, enabled or reordered.
#[derive(Default)]
pub struct PluginGroupBuilder {
    plugins: HashMap<TypeId, PluginEntry>,
    order: Vec<TypeId>,
}

impl PluginGroupBuilder {
    /// Finds the index of a target [`Plugin`]. Panics if the target's [`TypeId`] is not found.
    fn index_of<Target: Plugin>(&mut self) -> usize {
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

    /// Adds the plugin [`Plugin`] at the end of this [`PluginGroupBuilder`]. If the plugin was
    /// already in the group, it is removed from its previous place.
    pub fn add<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        let target_index = self.order.len();
        self.order.push(TypeId::of::<T>());
        self.upsert_plugin_state(plugin, target_index);
        self
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] before the plugin of type `Target`.
    /// If the plugin was already the group, it is removed from its previous place. There must
    /// be a plugin of type `Target` in the group or it will panic.
    pub fn add_before<Target: Plugin, T: Plugin>(&mut self, plugin: T) -> &mut Self {
        let target_index = self.index_of::<Target>();
        self.order.insert(target_index, TypeId::of::<T>());
        self.upsert_plugin_state(plugin, target_index);
        self
    }

    /// Adds a [`Plugin`] in this [`PluginGroupBuilder`] after the plugin of type `Target`.
    /// If the plugin was already the group, it is removed from its previous place. There must
    /// be a plugin of type `Target` in the group or it will panic.
    pub fn add_after<Target: Plugin, T: Plugin>(&mut self, plugin: T) -> &mut Self {
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
    pub fn enable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
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
    pub fn disable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot disable a plugin that does not exist.");
        plugin_entry.enabled = false;
        self
    }

    /// Consumes the [`PluginGroupBuilder`] and [builds](Plugin::build) the contained [`Plugin`]s
    /// in the order specified.
    pub fn finish(self, app: &mut App) {
        for ty in &self.order {
            if let Some(entry) = self.plugins.get(ty) {
                if entry.enabled {
                    debug!("added plugin: {}", entry.plugin.name());
                    entry.plugin.build(app);
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
    fn build(&mut self, _: &mut PluginGroupBuilder) {}
}

#[cfg(test)]
mod tests {
    use super::PluginGroupBuilder;
    use crate::{App, Plugin};

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
        let mut group = PluginGroupBuilder::default();
        group.add(PluginA);
        group.add(PluginB);
        group.add(PluginC);

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
        let mut group = PluginGroupBuilder::default();
        group.add(PluginA);
        group.add(PluginB);
        group.add_after::<PluginA, PluginC>(PluginC);

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
        let mut group = PluginGroupBuilder::default();
        group.add(PluginA);
        group.add(PluginB);
        group.add_before::<PluginB, PluginC>(PluginC);

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
        let mut group = PluginGroupBuilder::default();
        group.add(PluginA);
        group.add(PluginB);
        group.add(PluginC);
        group.add(PluginB);

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
        let mut group = PluginGroupBuilder::default();
        group.add(PluginA);
        group.add(PluginB);
        group.add(PluginC);
        group.add_after::<PluginA, PluginC>(PluginC);

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
        let mut group = PluginGroupBuilder::default();
        group.add(PluginA);
        group.add(PluginB);
        group.add(PluginC);
        group.add_before::<PluginB, PluginC>(PluginC);

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
