use bevy_utils::HashMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use crate::App;
use crate::Plugin;

/// Plugins state in the application
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginState {
    /// Plugin is not initialized.
    #[default]
    Idle,
    /// Plugin is initialized.
    Init,
    /// Plugin is being built.
    Building,
    /// Plugin is not yet ready.
    NotYetReady,
    /// Plugin configuration is finishing.
    Finishing,
    /// Plugin configuration is completed.
    Done,
    /// Plugin resources are cleaned up.
    Cleaned,
}

impl PluginState {
    pub(crate) fn next(self) -> Self {
        match self {
            Self::Idle => Self::Init,
            Self::Init => Self::Building,
            Self::Building => Self::NotYetReady,
            Self::NotYetReady => Self::NotYetReady,
            Self::Finishing => Self::Done,
            s => unreachable!("Cannot handle {:?} state", s),
        }
    }
}

/// Plugins state in the application
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginRegistryState {
    #[default]
    /// No plugin has been added.
    Idle,
    /// Plugins are initialized.
    Init,
    /// Plugins are being built.
    Building,
    /// Plugins are being finalized.
    Finalizing,
    /// Plugins configuration is complete.
    Done,
    /// Plugins resources are cleaned up.
    Cleaned,
}

#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_states: HashMap<String, PluginState>,
    state: PluginRegistryState,
}

impl PluginRegistry {
    pub fn state(&self) -> PluginRegistryState {
        self.state
    }

    pub fn add(&mut self, plugin: Box<dyn Plugin>) {
        if self.state() >= PluginRegistryState::Finalizing {
            panic!("Cannot add plugins after the ready state");
        }

        let name = plugin.name().to_string();

        self.plugin_states.insert(name, PluginState::Idle);
        self.plugins.push(plugin);
        self.update_state();
    }

    pub fn get_all<T: Plugin>(&self) -> Vec<&T> {
        self.plugins
            .iter()
            .filter_map(|p| p.downcast_ref())
            .collect()
    }

    pub fn contains<T: Plugin>(&self) -> bool {
        self.get::<T>().is_some()
    }

    pub fn get<T: Plugin>(&self) -> Option<&T> {
        for p in &self.plugins {
            if let Some(t) = p.downcast_ref() {
                return Some(t);
            }
        }

        None
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn plugin_state(&self, name: &str) -> Option<PluginState> {
        self.plugin_states.get(name).cloned()
    }

    pub fn update(&mut self, app: &mut App) {
        for plugin in &mut self.plugins {
            let current_state = self
                .plugin_states
                .get_mut(plugin.name())
                .expect("Plugin state must exist");

            if *current_state < PluginState::Done {
                let mut next_state = current_state.next();
                if next_state == PluginState::NotYetReady {
                    if !plugin.ready(app) {
                        *current_state = next_state;
                        continue;
                    }

                    next_state = PluginState::Finishing;
                }

                let result = catch_unwind(AssertUnwindSafe(|| {
                    plugin.update(app, next_state);
                }));

                if let Err(payload) = result {
                    resume_unwind(payload);
                }

                *current_state = next_state;
            }
        }

        self.update_state();
    }

    fn update_state(&mut self) {
        self.state = self
            .plugin_states
            .values()
            .min()
            .map(|s| match s {
                PluginState::Idle | PluginState::Init => PluginRegistryState::Init,
                PluginState::Building | PluginState::NotYetReady => PluginRegistryState::Building,
                PluginState::Finishing => PluginRegistryState::Finalizing,
                PluginState::Done => PluginRegistryState::Done,
                PluginState::Cleaned => PluginRegistryState::Cleaned,
            })
            .unwrap_or(PluginRegistryState::Idle);
    }

    pub fn cleanup(&mut self, app: &mut App) {
        for plugin in &mut self.plugins {
            let current_state = self
                .plugin_states
                .get_mut(plugin.name())
                .expect("Plugin state must exist");

            if *current_state != PluginState::Done {
                panic!(
                    "Cannot cleanup a not-finalized plugin: {} (current state: {:?})",
                    plugin.name(),
                    current_state
                );
            }

            let result = catch_unwind(AssertUnwindSafe(|| {
                plugin.cleanup(app);
            }));

            if let Err(payload) = result {
                resume_unwind(payload);
            }

            *current_state = PluginState::Cleaned;
        }

        self.update_state();
    }

    pub fn merge(&mut self, mut other: Self) {
        other.plugins.extend(self.plugins.drain(..));
        other.plugin_states.extend(self.plugin_states.drain());

        self.plugins = other.plugins;
        self.plugin_states = other.plugin_states;
        self.update_state();
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::prelude::Resource;

    use super::*;

    #[derive(Clone, Copy, Debug, Default, Resource)]
    pub struct TestResource {
        init: usize,
        built: usize,
        finished: usize,
        cleaned: usize,
    }

    #[derive(Clone)]
    pub struct TestPlugin;

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "TestPlugin"
        }

        fn init(&self, app: &mut App) {
            let mut res = TestResource::default();
            res.init += 1;

            app.world_mut().insert_resource(res);
        }

        fn build(&self, app: &mut App) {
            let mut res = app.world_mut().resource_mut::<TestResource>();
            res.built += 1;
        }

        fn finalize(&self, app: &mut App) {
            let mut res = app.world_mut().resource_mut::<TestResource>();
            res.finished += 1;
        }

        fn cleanup(&self, app: &mut App) {
            let mut res = app.world_mut().resource_mut::<TestResource>();
            res.cleaned += 1;
        }
    }

    #[derive(Clone)]
    pub struct DummyPlugin;

    impl Plugin for DummyPlugin {
        fn build(&self, _app: &mut App) {}
    }

    #[derive(Clone)]
    pub struct PanicPlugin;

    impl Plugin for PanicPlugin {
        fn build(&self, app: &mut App) {
            app.run();
        }
    }

    #[test]
    fn test_empty() {
        let registry = PluginRegistry::default();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
        assert_eq!(registry.state(), PluginRegistryState::Idle);
        assert!(!registry.contains::<TestPlugin>());
        assert_eq!(registry.plugin_state("TestPlugin"), None);
    }

    #[test]
    fn test_add() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin));

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        assert_eq!(registry.state(), PluginRegistryState::Init);

        assert!(registry.contains::<TestPlugin>());
        let plugins = registry.get_all::<TestPlugin>();
        assert_eq!(plugins.len(), 1);

        let plugin = registry.get::<TestPlugin>().unwrap();
        assert_eq!(plugin.name(), "TestPlugin");
    }

    #[test]
    fn test_update() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin));

        let mut app = App::new();

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Init);
        assert_eq!(registry.plugin_state("TestPlugin"), Some(PluginState::Init));
        assert_plugin_status(&app, 1, 0, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Building);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::Building)
        );
        assert_plugin_status(&app, 1, 1, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Finalizing);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::Finishing)
        );
        assert_plugin_status(&app, 1, 1, 1, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Done);
        assert_eq!(registry.plugin_state("TestPlugin"), Some(PluginState::Done));
        assert_plugin_status(&app, 1, 1, 1, 0);
    }

    #[test]
    fn test_cleanup() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin));

        let mut app = App::new();

        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        assert_eq!(registry.plugin_state("TestPlugin"), Some(PluginState::Done));

        registry.cleanup(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Cleaned);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::Cleaned)
        );
        assert_plugin_status(&app, 1, 1, 1, 1);
    }

    #[test]
    #[should_panic]
    fn cannot_cleanup_a_non_finalized_plugin() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin));

        let mut app = App::new();

        registry.cleanup(&mut app);
    }

    #[derive(Clone)]
    pub struct WaitingPlugin {
        ready: bool,
    }

    impl Plugin for WaitingPlugin {
        fn build(&self, _app: &mut App) {}
        fn ready(&self, _app: &App) -> bool {
            self.ready
        }
    }

    #[test]
    fn check_ready() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(WaitingPlugin { ready: false }));

        let mut app = App::new();

        registry.update(&mut app);
        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Building);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Building);
    }

    #[test]
    #[should_panic]
    fn plugins_cannot_be_added_after_being_ready() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin));

        let mut app = App::new();
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);

        assert_eq!(registry.state(), PluginRegistryState::Finalizing);

        registry.add(Box::new(DummyPlugin));
    }

    fn assert_plugin_status(app: &App, init: usize, built: usize, finished: usize, cleaned: usize) {
        let res = app.world().resource::<TestResource>();

        assert_eq!(res.init, init, "Wrong init status");
        assert_eq!(res.built, built, "Wrong built status");
        assert_eq!(res.finished, finished, "Wrong finished status");
        assert_eq!(res.cleaned, cleaned, "Wrong cleaned status");
    }
}
