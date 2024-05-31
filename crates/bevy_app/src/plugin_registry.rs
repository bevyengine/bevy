use bevy_utils::HashMap;
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

use crate::app::AppError;
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
    plugin_states: HashMap<usize, PluginState>,
    plugin_names: HashMap<usize, String>,
    state: PluginRegistryState,
}

impl PluginRegistry {
    /// Returns the registry current state
    pub fn state(&self) -> PluginRegistryState {
        self.state
    }

    /// Add a new plugin. Plugins can be added only before the finalizing state.
    pub(crate) fn add(&mut self, plugin: Box<dyn Plugin>) -> Result<(), AppError> {
        if self.state() >= PluginRegistryState::Finalizing {
            panic!("Cannot add plugins after the ready state");
        }

        self.add_with_state(plugin, PluginState::Idle)
    }

    fn add_with_state(
        &mut self,
        plugin: Box<dyn Plugin>,
        state: PluginState,
    ) -> Result<(), AppError> {
        if !self.allow_duplicate(&plugin) {
            return Err(AppError::DuplicatePlugin {
                plugin_name: plugin.name().to_string(),
            })?;
        }

        let index = self.plugins.len();
        let name = plugin.name().to_string();

        self.plugin_states.insert(index, state);
        self.plugin_names.insert(index, name);
        self.plugins.push(plugin);
        self.update_state();

        Ok(())
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
        for (id, plugin) in &mut self.plugins.iter_mut().enumerate() {
            let current_state = self
                .plugin_states
                .get_mut(&id)
                .expect("Plugin state must exist");

            if *current_state < PluginState::Done {
                let next_state = current_state.next();

                if next_state == PluginState::Finalizing && !plugin.check_required_sub_apps(app) {
                    *current_state = PluginState::Done;
                    continue;
                }

                if !is_plugin_ready(plugin, app, next_state) {
                    continue;
                }

                let result = catch_unwind(AssertUnwindSafe(|| {
                    process_plugin(plugin, app, next_state);
                }));

                if let Err(payload) = result {
                    resume_unwind(payload);
                }

                *current_state = next_state;
            }
        }

        self.update_state();
    }

    pub(crate) fn cleanup(&mut self, app: &mut App) {
        for (id, plugin) in &mut self.plugins.iter_mut().enumerate() {
            let current_state = self
                .plugin_states
                .get_mut(&id)
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

    pub(crate) fn merge(&mut self, other: Self) -> Result<(), AppError> {
        for (id, plugin) in other.plugins.into_iter().enumerate() {
            let state = other.plugin_states[&id];
            self.add_with_state(plugin, state)?;
        }

        Ok(())
    }

    fn id_by_name(&self, needle: &str) -> Option<usize> {
        self.plugin_names.iter().find_map(
            |(id, name)| {
                if name == needle {
                    Some(*id)
                } else {
                    None
                }
            },
        )
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

    fn allow_duplicate(&self, plugin: &Box<dyn Plugin>) -> bool {
        if let Some(id) = self.id_by_name(plugin.name()) {
            let ref existing = self.plugins[id];
            if plugin.is_unique() || existing.is_unique() {
                return false;
            }
        }

        true
    }
}

/// Process the plugin to a desired [`PluginState`].
fn process_plugin(plugin: &mut Box<dyn Plugin>, app: &mut App, state: PluginState) {
    match state {
        PluginState::Init => plugin.init(app),
        PluginState::SettingUp => plugin.setup(app),
        PluginState::Configuring => plugin.configure(app),
        PluginState::Finalizing => plugin.finalize(app),
        PluginState::Done => {}
        s => panic!("Cannot handle {s:?} state during plugin processing"),
    }
}

/// Checks if the plugin is ready to progress to the desired next [`PluginState`].
fn is_plugin_ready(plugin: &Box<dyn Plugin>, app: &mut App, next_state: PluginState) -> bool {
    match next_state {
        PluginState::SettingUp => plugin.ready_to_setup(app),
        PluginState::Configuring => plugin.ready_to_configure(app),
        PluginState::Finalizing => plugin.ready_to_finalize(app),
        _ => true,
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
        finalized: usize,
        cleaned: usize,
    }

    #[derive(Clone)]
    pub struct TestPlugin {
        require_sub_app: bool,
        unique: bool,
    }

    impl Default for TestPlugin {
        fn default() -> Self {
            Self {
                require_sub_app: false,
                unique: true,
            }
        }
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
            if !app.world_mut().contains_resource::<TestResource>() {
                app.init_resource::<TestResource>();
            }

            let mut res = app.world_mut().resource_mut::<TestResource>();
            res.init += 1;
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
            res.finalized += 1;
        }

        fn cleanup(&self, app: &mut App) {
            let mut res = app.world_mut().resource_mut::<TestResource>();
            res.cleaned += 1;
        }

        fn is_unique(&self) -> bool {
            self.unique
        }
    }

    impl TestPlugin {
        fn non_unique(mut self) -> Self {
            self.unique = false;
            self
        }

        fn require_sub_app(mut self) -> Self {
            self.require_sub_app = true;
            self
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
        fn plugin_states(&self, needle: &str) -> HashMap<usize, PluginState> {
            let vec = self
                .plugin_names
                .iter()
                .filter_map(|(id, name)| {
                    if name == needle {
                        Some((*id, self.plugin_states[id]))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            vec.into_iter().collect()
        }
    }

    #[test]
    fn test_empty() {
        let registry = PluginRegistry::default();
        assert_eq!(registry.plugins.len(), 0);
        assert_eq!(registry.state(), PluginRegistryState::Idle);
        assert!(!registry.contains::<TestPlugin>());
        assert_plugin_states(&registry, "TestPlugin", vec![]);
    }

    #[test]
    fn test_add() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::<TestPlugin>::default());

        assert_eq!(registry.plugins.len(), 1);

        assert_eq!(registry.state(), PluginRegistryState::Init);

        assert!(registry.contains::<TestPlugin>());
        let plugins = registry.get_all::<TestPlugin>();
        assert_eq!(plugins.len(), 1);

        let plugin = registry.get::<TestPlugin>().unwrap();
        assert_eq!(plugin.name(), "TestPlugin");
    }

    #[test]
    fn test_cannot_add_unique_twice() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::<TestPlugin>::default());

        assert!(registry.add(Box::<TestPlugin>::default()).is_err());

        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::new(TestPlugin::default().non_unique()));

        assert!(registry.add(Box::<TestPlugin>::default()).is_err());
    }

    #[test]
    fn test_update() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Init);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Init)]);
        assert_plugin_state(&app, 1, 0, 0, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::SettingUp);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::SettingUp)]);
        assert_plugin_state(&app, 1, 1, 0, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Configuring);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Configuring)]);
        assert_plugin_state(&app, 1, 1, 1, 0, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Finalizing);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Finalizing)]);
        assert_plugin_state(&app, 1, 1, 1, 1, 0);

        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Done);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Done)]);
        assert_plugin_state(&app, 1, 1, 1, 1, 0);
    }

    #[test]
    fn test_update_non_unique() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::new(TestPlugin::default().non_unique()));

        let mut app = App::new();

        registry.update(&mut app);
        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::SettingUp);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::SettingUp)]);
        assert_plugin_state(&app, 1, 1, 0, 0, 0);

        let _ = registry.add(Box::new(TestPlugin::default().non_unique()));
        assert_eq!(registry.plugins.len(), 2);
        assert_eq!(registry.state(), PluginRegistryState::Init);
        assert_plugin_states(
            &registry,
            "TestPlugin",
            vec![(0, PluginState::SettingUp), (1, PluginState::Idle)],
        );

        assert_plugin_state(&app, 1, 1, 0, 0, 0);

        registry.update(&mut app);
        registry.update(&mut app);
        assert_plugin_state(&app, 2, 2, 1, 1, 0);

        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Done);
        assert_plugin_states(
            &registry,
            "TestPlugin",
            vec![(0, PluginState::Done), (1, PluginState::Done)],
        );
        assert_plugin_state(&app, 2, 2, 2, 2, 0);

        registry.cleanup(&mut app);
        assert_plugin_state(&app, 2, 2, 2, 2, 2);

        assert_eq!(registry.state(), PluginRegistryState::Cleaned);
    }

    #[test]
    fn test_cleanup() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();

        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Done)]);

        registry.cleanup(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Cleaned);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Cleaned)]);
        assert_plugin_state(&app, 1, 1, 1, 1, 1);
    }

    #[derive(Clone, Default)]
    pub struct DummyPluginA;

    impl Plugin for DummyPluginA {}

    #[derive(Clone, Default)]
    pub struct DummyPluginB;

    impl Plugin for DummyPluginB {}

    #[test]
    fn test_merge() {
        let plugin_a = Box::<DummyPluginA>::default();
        let plugin_b = Box::<DummyPluginB>::default();

        let mut app = App::new();

        let mut registry1 = PluginRegistry::default();
        let _ = registry1.add(plugin_a.clone());
        registry1.update(&mut app);

        let mut registry2 = PluginRegistry::default();
        let _ = registry2.add(plugin_b.clone());
        registry2.update(&mut app);

        let _ = registry1.merge(registry2);
        assert_eq!(registry1.plugins.len(), 2);
        assert_eq!(registry1.plugin_states.len(), 2);
        assert_eq!(registry1.plugin_names.len(), 2);

        assert_plugin_states(&registry1, plugin_a.name(), vec![(0, PluginState::Init)]);
        assert_plugin_states(&registry1, plugin_b.name(), vec![(1, PluginState::Init)]);
    }

    #[test]
    fn cant_merge_unique_plugin_twice() {
        let mut registry1 = PluginRegistry::default();
        let _ = registry1.add(Box::<DummyPluginA>::default());

        let mut registry2 = PluginRegistry::default();
        let _ = registry2.add(Box::<DummyPluginA>::default());

        assert!(registry1.merge(registry2).is_err());
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
        let _ = registry.add(Box::new(TestPlugin::default().require_sub_app()));

        let mut app = App::new();

        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);
        registry.cleanup(&mut app);

        assert_eq!(registry.state(), PluginRegistryState::Cleaned);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Cleaned)]);
        assert_plugin_state(&app, 1, 1, 1, 0, 1);
    }

    #[test]
    fn finalize_plugin_with_required_subapp() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::new(TestPlugin::default().require_sub_app()));
        let _ = registry.add(Box::new(SubAppCreatorPlugin));

        let mut app = App::new();

        registry.update(&mut app); // Init
        registry.update(&mut app); // Build
        registry.update(&mut app); // Configure
        registry.update(&mut app); // Finalize
        registry.update(&mut app); // Done
        registry.cleanup(&mut app);

        assert_eq!(registry.state(), PluginRegistryState::Cleaned);
        assert_plugin_states(&registry, "TestPlugin", vec![(0, PluginState::Cleaned)]);
        assert_plugin_state(&app, 1, 1, 1, 1, 1);
    }

    #[test]
    #[should_panic]
    fn cannot_cleanup_a_non_finalized_plugin() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();

        registry.cleanup(&mut app);
    }

    #[derive(Clone)]
    pub struct WaitingPlugin {
        ready: bool,
    }

    impl Plugin for WaitingPlugin {
        fn ready_to_setup(&self, _app: &mut App) -> bool {
            self.ready
        }
    }

    #[test]
    fn check_ready() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::new(WaitingPlugin { ready: false }));

        let mut app = App::new();

        registry.update(&mut app);
        registry.update(&mut app);
        assert_eq!(registry.state(), PluginRegistryState::Init);
    }

    #[test]
    #[should_panic]
    fn plugins_cannot_be_added_after_being_ready() {
        let mut registry = PluginRegistry::default();
        let _ = registry.add(Box::<TestPlugin>::default());

        let mut app = App::new();
        registry.update(&mut app);
        registry.update(&mut app);
        registry.update(&mut app);

        assert_eq!(registry.state(), PluginRegistryState::Finalizing);

        let _ = registry.add(Box::new(DummyPlugin));
    }

    fn assert_plugin_state(
        app: &App,
        init: usize,
        built: usize,
        configured: usize,
        finalized: usize,
        cleaned: usize,
    ) {
        let res = app.world().resource::<TestResource>();

        assert_eq!(res.init, init, "Wrong init status");
        assert_eq!(res.setup, built, "Wrong built status");
        assert_eq!(res.configured, configured, "Wrong configured status");
        assert_eq!(res.finalized, finalized, "Wrong finalized status");
        assert_eq!(res.cleaned, cleaned, "Wrong cleaned status");
    }

    fn assert_plugin_states(
        registry: &PluginRegistry,
        name: &str,
        states: Vec<(usize, PluginState)>,
    ) {
        let states = states.into_iter().collect::<HashMap<_, _>>();
        assert_eq!(registry.plugin_states(name), states);
        assert_eq!(registry.plugin_states(name).len(), states.len());
    }
}
