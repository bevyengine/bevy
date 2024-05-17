use bevy_utils::HashMap;

use crate::App;
use crate::Plugin;

/// Plugins state in the application
#[derive(PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum PluginState {
    /// Plugin is not initialized.
    Idle,
    /// Plugin is initialized.
    Init,
    /// Plugin is being built.
    Building,
    /// Plugin is not yet ready.
    NotYetReady,
    /// Plugin configuration is finished.
    Finished,
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
            Self::Finished => Self::Cleaned,
            _ => unreachable!()
        }
    }
}

/// Plugins state in the application
#[derive(PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub enum PluginsState {
    /// No plugin has been added.
    None,
    /// Plugins are initialized.
    Init,
    /// Plugins are being built.
    Building,
    /// Plugins are being finalized.
    Finalizing,
    /// Plugins configuration is complete.
    Done,
}

#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    states: HashMap<String, PluginState>,
}

impl PluginRegistry {
    pub fn add(&mut self, plugin: Box<dyn Plugin>) {
        if self.state() >= PluginsState::Finalizing {
            panic!("Cannot add plugins after the ready state");
        }

        let name = plugin.name().to_string();

        self.states.insert(name, PluginState::Idle);
        self.plugins.push(plugin);
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

    pub fn state(&self) -> PluginsState {
        self.states.values().min().map(|s| {
            match s {
                PluginState::Idle | PluginState::Init => PluginsState::Init,
                PluginState::Building | PluginState::NotYetReady  => PluginsState::Building,
                PluginState::Finished => PluginsState::Finalizing,
                PluginState::Cleaned => PluginsState::Done,
            }
        }).unwrap_or(PluginsState::None)
    }

    pub fn update(&mut self, app: &mut App) {
        println!("Registry state before: {:?} ({})", self.state(), self.plugins.len());

        for plugin in &mut self.plugins {
            let current_state = self.states.get_mut(plugin.name()).expect("Plugin state must exist");

            if *current_state < PluginState::Cleaned {
                let mut next_state = current_state.next();
                if next_state == PluginState::NotYetReady {
                    if !plugin.ready(app) {
                        println!("Plugin {} not ready yet", plugin.name());

                        *current_state = next_state;
                        continue;
                    }

                    next_state = PluginState::Finished;
                }
                println!("Updating {} to {next_state:?}", plugin.name());

                plugin.update(app, next_state);
                *current_state = next_state;
            }
        }

        println!("Registry state after: {:?} ({})", self.state(), self.plugins.len());
    }

    pub fn merge(&mut self, mut other: Self) {
        other.plugins.extend(self.plugins.drain(..));
        other.states.extend(self.states.drain());

        self.plugins = other.plugins;
        self.states = other.states;
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

        fn finish(&self, app: &mut App) {
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
        assert_eq!(registry.state(), PluginsState::None);
        assert!(!registry.contains::<TestPlugin>());
    }

    #[test]
    fn test_add() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin));

        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        assert_eq!(registry.state(), PluginsState::Init);

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
        assert_eq!(registry.state(), PluginsState::Init);
        assert_plugin_status(&app, 1, 0, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginsState::Building);
        assert_plugin_status(&app, 1, 1, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginsState::Finalizing);
        assert_plugin_status(&app, 1, 1, 1, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginsState::Done);
        assert_plugin_status(&app, 1, 1, 1, 1);
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
        assert_eq!(registry.state(), PluginsState::Building);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginsState::Building);
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

        assert_eq!(registry.state(), PluginsState::Finalizing);

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