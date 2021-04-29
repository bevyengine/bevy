use crate::{AppBuilder, Plugin};
use bevy_utils::{tracing::debug, tracing::warn, HashMap};
use std::any::TypeId;

pub trait PluginGroup {
    fn build(&mut self, group: &mut PluginGroupBuilder);
}

struct PluginEntry {
    plugin: Box<dyn Plugin>,
    enabled: bool,
}

/// Builds and customizes a plugin group. A plugin group is an ordered list of plugins that
/// that can be enabled, disabled or reordered.
#[derive(Default)]
pub struct PluginGroupBuilder {
    plugins: HashMap<TypeId, PluginEntry>,
    order: Vec<TypeId>,
}

impl PluginGroupBuilder {
    // Removes a previous ordering of a plugin that has just been added at `added_at` index
    fn remove_when_adding<T: Plugin>(&mut self, added_at: usize) {
        if let Some(to_remove) = self
            .order
            .iter()
            .enumerate()
            .find(|(i, ty)| *i != added_at && **ty == TypeId::of::<T>())
            .map(|(i, _)| i)
        {
            self.order.remove(to_remove);
        }
    }

    /// Adds the plugin `plugin` at the end of this `PluginGroupBuilder`. If the plugin was
    /// already in the group, it is removed from its previous place.
    pub fn add<T: Plugin>(&mut self, plugin: T) -> &mut Self {
        let target_index = self.order.len();
        self.order.push(TypeId::of::<T>());
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
            self.remove_when_adding::<T>(target_index);
        }

        self
    }

    /// Adds the plugin `plugin` in this `PluginGroupBuilder` before the plugin of type `Target`.
    /// If the plugin was already the group, it is removed from its previous place. There must
    /// be a plugin of type `Target` in the group or it will fail.
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
            self.remove_when_adding::<T>(target_index);
        }
        self
    }

    /// Adds the plugin `plugin` in this `PluginGroupBuilder` after the plugin of type `Target`.
    /// If the plugin was already the group, it is removed from its previous place. There must
    /// be a plugin of type `Target` in the group or it will fail.
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
            })
            + 1;
        self.order.insert(target_index, TypeId::of::<T>());
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
            self.remove_when_adding::<T>(target_index);
        }
        self
    }

    /// Enables the plugin of type `T` in this `PluginGroupBuilder`. There must
    /// be a plugin of type `Target` in the group or it will fail.
    pub fn enable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot enable a plugin that does not exist.");
        plugin_entry.enabled = true;
        self
    }

    /// Disables the plugin of type `T` in this `PluginGroupBuilder`, but leave it in its
    /// place so that you can still use [`add_before`](Self::add_before) or
    /// [`add_after`](Self::add_after), or re-enable it with [`enable`](Self::enable).
    /// There must be a plugin of type `Target` in the group or it will fail.
    pub fn disable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot disable a plugin that does not exist.");
        plugin_entry.enabled = false;
        self
    }

    /// Adds the enabled [`Plugin`] from this group in order to the application.
    pub fn finish(self, app: &mut AppBuilder) {
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

#[cfg(test)]
mod tests {
    use super::PluginGroupBuilder;
    use crate::{AppBuilder, Plugin};

    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, _: &mut AppBuilder) {}
    }

    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, _: &mut AppBuilder) {}
    }

    struct PluginC;
    impl Plugin for PluginC {
        fn build(&self, _: &mut AppBuilder) {}
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
        )
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
        )
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
        )
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
        )
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
        )
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
        )
    }
}
