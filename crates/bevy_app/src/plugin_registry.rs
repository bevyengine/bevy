use bevy_utils::HashMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use crate::Plugin;
use crate::{App, PluginState};

/// Plugin registry state in the application
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum PluginRegistryState {
    #[default]
    /// No plugin has been added.
    Idle,
    /// Plugins are being initialized.
    Init,
    /// Plugins are being set up.
    SettingUp,
    /// Plugins are being configured.
    Configuring,
    /// Plugins are being finalized.
    Finalizing,
    /// Plugin configuration is complete.
    Done,
    /// Plugins resources are cleaned up.
    Cleaned,
}

/// Registry for all the [`App`] [`Plugin`]s
#[derive(Default)]
pub struct PluginRegistry {
    plugins: Vec<Box<dyn Plugin>>,
    plugin_states: HashMap<String, PluginState>,
    state: PluginRegistryState,
}

impl PluginRegistry {
    /// Returns the registry current state
    pub fn state(&self) -> PluginRegistryState {
        self.state
    }

    /// Add a new plugin. Plugins can be added only before the finalizing state.
    pub(crate) fn add(&mut self, plugin: Box<dyn Plugin>) {
        if self.state() >= PluginRegistryState::Finalizing {
            panic!("Cannot add plugins after the ready state");
        }

        let name = plugin.name().to_string();

        self.plugin_states.insert(name, PluginState::Idle);
        self.plugins.push(plugin);
        self.update_state();
    }

    /// Returns all the plugin of a specified type.
    pub fn get_all<T: Plugin>(&self) -> Vec<&T> {
        self.plugins
            .iter()
            .filter_map(|p| p.downcast_ref())
            .collect()
    }

    /// Returns `true` if the registry contains the required plugin by type
    pub fn contains<T: Plugin>(&self) -> bool {
        self.get::<T>().is_some()
    }

    /// Returns a reference to the plugin of type `T` if it exists
    pub fn get<T: Plugin>(&self) -> Option<&T> {
        for p in &self.plugins {
            if let Some(t) = p.downcast_ref() {
                return Some(t);
            }
        }

        None
    }

    /// Updates all plugins up to the [`PluginState::Done`] state.
    pub(crate) fn update(&mut self, app: &mut App) {
        for plugin in &mut self.plugins {
            let current_state = self
                .plugin_states
                .get_mut(plugin.name())
                .expect("Plugin state must exist");

            if *current_state < PluginState::Done {
                let next_state = current_state.next();

                if next_state == PluginState::Finalizing && !plugin.check_required_sub_apps(app) {
                    *current_state = PluginState::Done;
                    continue;
                }

                if !plugin.ready(app, next_state) {
                    continue;
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
                PluginState::SettingUp => PluginRegistryState::SettingUp,
                PluginState::Finalizing => PluginRegistryState::Finalizing,
                PluginState::Configuring => PluginRegistryState::Configuring,
                PluginState::Done => PluginRegistryState::Done,
                PluginState::Cleaned => PluginRegistryState::Cleaned,
            })
            .unwrap_or(PluginRegistryState::Idle);
    }

    pub(crate) fn cleanup(&mut self, app: &mut App) {
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

    pub(crate) fn merge(&mut self, mut other: Self) {
        other.plugins.append(&mut self.plugins);
        other.plugin_states.extend(self.plugin_states.drain());

        self.plugins = other.plugins;
        self.plugin_states = other.plugin_states;
        self.update_state();
    }
}

#[cfg(test)]
mod tests {
    use crate::{self as bevy_app};
    use crate::{AppLabel, InternedAppLabel, SubApp};
    use bevy_ecs::prelude::Resource;

    use super::*;

    #[derive(Clone, Copy, Debug, Default, Resource)]
    pub struct TestResource {
        init: usize,
        setup: usize,
        configured: usize,
        finished: usize,
        cleaned: usize,
    }

    #[derive(Clone, Default)]
    pub struct TestPlugin {
        require_sub_app: bool,
    }

    impl Plugin for TestPlugin {
        fn name(&self) -> &str {
            "TestPlugin"
        }

        fn required_sub_apps(&self) -> Vec<InternedAppLabel> {
            if self.require_sub_app {
                return vec![DummyApp.intern()];
            }

            Vec::new()
        }

        fn init(&self, app: &mut App) {
            let mut res = TestResource::default();
            res.init += 1;

            app.world_mut().insert_resource(res);
        }

        fn setup(&self, app: &mut App) {
            let mut res = app.world_mut().resource_mut::<TestResource>();
            res.setup += 1;
        }

        fn configure(&self, app: &mut App) {
            let mut res = app.world_mut().resource_mut::<TestResource>();
            res.configured += 1;
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

    impl Plugin for DummyPlugin {}

    #[derive(Clone)]
    pub struct PanicPlugin;

    impl Plugin for PanicPlugin {
        fn init(&self, app: &mut App) {
            app.run();
        }
    }

    impl PluginRegistry {
        fn plugin_state(&self, name: &str) -> Option<PluginState> {
            self.plugin_states.get(name).cloned()
        }
    }

    #[test]
    fn test_empty() {
        let registry = PluginRegistry::default();
        assert_eq!(registry.plugins.len(), 0);
        assert_eq!(registry.state(), PluginRegistryState::Idle);
        assert!(!registry.contains::<TestPlugin>());
        assert_eq!(registry.plugin_state("TestPlugin"), None);
    }

    #[test]
    fn test_add() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::<TestPlugin>::default());

        assert_eq!(registry.plugins.len(), 1);

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
        registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Init);
        assert_eq!(registry.plugin_state("TestPlugin"), Some(PluginState::Init));
        assert_plugin_status(&app, 1, 0, 0, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::SettingUp);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::SettingUp)
        );
        assert_plugin_status(&app, 1, 1, 0, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Configuring);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::Configuring)
        );
        assert_plugin_status(&app, 1, 1, 1, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Finalizing);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::Finalizing)
        );
        assert_plugin_status(&app, 1, 1, 1, 1, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Done);
        assert_eq!(registry.plugin_state("TestPlugin"), Some(PluginState::Done));
        assert_plugin_status(&app, 1, 1, 1, 1, 0);
    }

    #[test]
    fn test_cleanup() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();

        registry.update(&mut app);
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
        assert_plugin_status(&app, 1, 1, 1, 1, 1);
    }

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
    pub struct DummyApp;

    #[derive(Clone)]
    pub struct SubAppCreatorPlugin;

    impl Plugin for SubAppCreatorPlugin {
        fn init(&self, app: &mut App) {
            app.insert_sub_app(DummyApp, SubApp::new("dummy"));
        }
    }

    #[test]
    fn dont_finalize_plugin_without_required_subapp() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin {
            require_sub_app: true,
        }));

        let mut app = App::new();

        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.cleanup(&mut app);

        assert_eq!(registry.state(), PluginRegistryState::Cleaned);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::Cleaned)
        );
        assert_plugin_status(&app, 1, 1, 1, 0, 1);
    }

    #[test]
    fn finalize_plugin_with_required_subapp() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::new(TestPlugin {
            require_sub_app: true,
        }));
        registry.add(Box::new(SubAppCreatorPlugin));

        let mut app = App::new();

        registry.update(&mut app); // Init
        registry.update(&mut app); // Build
        registry.update(&mut app); // Configure
        registry.update(&mut app); // Finalize
        registry.update(&mut app); // Done
        registry.cleanup(&mut app);

        assert_eq!(registry.state(), PluginRegistryState::Cleaned);
        assert_eq!(
            registry.plugin_state("TestPlugin"),
            Some(PluginState::Cleaned)
        );
        assert_plugin_status(&app, 1, 1, 1, 1, 1);
    }

    #[test]
    #[should_panic]
    fn cannot_cleanup_a_non_finalized_plugin() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();

        registry.cleanup(&mut app);
    }

    #[derive(Clone)]
    pub struct WaitingPlugin {
        ready: bool,
    }

    impl Plugin for WaitingPlugin {
        fn ready_to_build(&self, _app: &mut App) -> bool {
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
        assert_eq!(registry.state(), PluginRegistryState::Init);
    }

    #[test]
    #[should_panic]
    fn plugins_cannot_be_added_after_being_ready() {
        let mut registry = PluginRegistry::default();
        registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);

        assert_eq!(registry.state(), PluginRegistryState::Finalizing);

        registry.add(Box::new(DummyPlugin));
    }

    fn assert_plugin_status(
        app: &App,
        init: usize,
        built: usize,
        configured: usize,
        finished: usize,
        cleaned: usize,
    ) {
        let res = app.world().resource::<TestResource>();

        assert_eq!(res.init, init, "Wrong init status");
        assert_eq!(res.setup, built, "Wrong built status");
        assert_eq!(res.configured, configured, "Wrong configured status");
        assert_eq!(res.finished, finished, "Wrong finished status");
        assert_eq!(res.cleaned, cleaned, "Wrong cleaned status");
    }
}
