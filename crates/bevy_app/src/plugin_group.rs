use crate::{AppBuilder, Plugin};
use bevy_utils::{tracing::debug, HashMap};
use std::any::TypeId;

pub trait PluginGroup {
    fn build(&mut self, group: &mut PluginGroupBuilder);
}

struct PluginEntry {
    plugin: Box<dyn Plugin>,
    enabled: bool,
}

#[derive(Default)]
pub struct PluginGroupBuilder {
    plugins: HashMap<TypeId, PluginEntry>,
    order: Vec<TypeId>,
}

impl PluginGroupBuilder {
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

    pub fn enable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot enable a plugin that does not exist.");
        plugin_entry.enabled = true;
        self
    }

    pub fn disable<T: Plugin>(&mut self) -> &mut Self {
        let mut plugin_entry = self
            .plugins
            .get_mut(&TypeId::of::<T>())
            .expect("Cannot disable a plugin that does not exist.");
        plugin_entry.enabled = false;
        self
    }

    pub fn remove<T: Plugin>(&mut self) -> &mut Self {
        let plugin_exists = self.plugins.contains_key(&TypeId::of::<T>());

        if plugin_exists {
            self.order.retain(|val| val != &TypeId::of::<T>());

            self.plugins
                .remove(&TypeId::of::<T>())
                .expect("Could not remove a plugin");
        }
        self
    }

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
    use super::*;

    struct TestPlugin1;
    impl Plugin for TestPlugin1 {
        fn build(&self, _app: &mut AppBuilder) {}
    }

    struct TestPlugin2;
    impl Plugin for TestPlugin2 {
        fn build(&self, _app: &mut AppBuilder) {}
    }

    struct TestPlugin3;
    impl Plugin for TestPlugin3 {
        fn build(&self, _app: &mut AppBuilder) {}
    }

    impl PluginGroupBuilder {
        fn contains<T: Plugin>(&self) -> bool {
            self.plugins.contains_key(&TypeId::of::<T>())
        }

        fn get_idx<T: Plugin>(&self) -> Option<usize> {
            self.order
                .iter()
                .enumerate()
                .find(|(_, &type_id)| type_id == TypeId::of::<T>())
                .map(|(idx, _)| idx)
        }
    }

    #[test]
    fn adds_plugin() {
        let mut builder = PluginGroupBuilder::default();
        builder.add(TestPlugin1);
        assert!(builder.contains::<TestPlugin1>());
        assert_eq!(builder.get_idx::<TestPlugin1>().unwrap(), 0);
    }

    #[test]
    fn adds_plugin_before() {
        let mut builder = PluginGroupBuilder::default();
        builder.add(TestPlugin1);
        builder.add_before::<TestPlugin1, _>(TestPlugin2);
        assert_eq!(builder.get_idx::<TestPlugin2>().unwrap(), 0);
        assert_eq!(builder.get_idx::<TestPlugin1>().unwrap(), 1);
    }

    #[test]
    fn adds_plugin_after() {
        let mut builder = PluginGroupBuilder::default();
        builder.add(TestPlugin1);
        builder.add(TestPlugin2);
        builder.add_after::<TestPlugin1, _>(TestPlugin3);
        assert_eq!(builder.get_idx::<TestPlugin1>().unwrap(), 0);
        assert_eq!(builder.get_idx::<TestPlugin2>().unwrap(), 2);
        assert_eq!(builder.get_idx::<TestPlugin3>().unwrap(), 1);
    }

    #[test]
    fn removes_plugin() {
        let mut builder = PluginGroupBuilder::default();
        builder.add(TestPlugin1);
        assert!(builder.contains::<TestPlugin1>());
        builder.remove::<TestPlugin1>();
        assert!(!builder.contains::<TestPlugin1>());
    }

    #[test]
    fn enables_and_disables_plugin() {
        let mut builder = PluginGroupBuilder::default();
        builder.add(TestPlugin1);

        assert_eq!(
            builder
                .plugins
                .get(&TypeId::of::<TestPlugin1>())
                .unwrap()
                .enabled,
            true
        );

        builder.disable::<TestPlugin1>();
        assert_eq!(
            builder
                .plugins
                .get(&TypeId::of::<TestPlugin1>())
                .unwrap()
                .enabled,
            false
        );

        builder.enable::<TestPlugin1>();
        assert_eq!(
            builder
                .plugins
                .get(&TypeId::of::<TestPlugin1>())
                .unwrap()
                .enabled,
            true
        );
    }
}
