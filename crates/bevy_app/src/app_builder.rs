use std::{any::Any, hash::Hash};

use crate::{
    app::{App, AppExit},
    event::Events,
    plugin::Plugin,
    stage, startup_stage, PluginGroup, PluginGroupBuilder,
};
use bevy_ecs::{
    FromResources, IntoSystem, Resource, Resources, Stage, State, StateStage, System, SystemStage,
    World,
};
use bevy_utils::tracing::debug;

/// Configure [App]s using the builder pattern
pub struct AppBuilder {
    pub app: App,
}

impl Default for AppBuilder {
    fn default() -> Self {
        let mut app_builder = AppBuilder {
            app: App::default(),
        };

        app_builder
            .add_default_stages()
            .add_event::<AppExit>()
            .add_system_to_stage(stage::LAST, clear_trackers_system);
        app_builder
    }
}

fn clear_trackers_system(world: &mut World, resources: &mut Resources) {
    world.clear_trackers();
    resources.clear_trackers();
}

impl AppBuilder {
    pub fn empty() -> AppBuilder {
        AppBuilder {
            app: App::default(),
        }
    }

    pub fn resources(&self) -> &Resources {
        &self.app.resources
    }

    pub fn resources_mut(&mut self) -> &mut Resources {
        &mut self.app.resources
    }

    pub fn run(&mut self) {
        let app = std::mem::take(&mut self.app);
        app.run();
    }

    pub fn set_world(&mut self, world: World) -> &mut Self {
        self.app.world = world;
        self
    }

    pub fn add_stage<S: Stage>(&mut self, name: &'static str, stage: S) -> &mut Self {
        self.app.schedule.add_stage(name, stage);
        self
    }

    pub fn add_stage_after<S: Stage>(
        &mut self,
        target: &'static str,
        name: &'static str,
        stage: S,
    ) -> &mut Self {
        self.app.schedule.add_stage_after(target, name, stage);
        self
    }

    pub fn add_stage_before<S: Stage>(
        &mut self,
        target: &'static str,
        name: &'static str,
        stage: S,
    ) -> &mut Self {
        self.app.schedule.add_stage_before(target, name, stage);
        self
    }

    pub fn add_startup_stage<S: Stage>(&mut self, name: &'static str, stage: S) -> &mut Self {
        self.app.startup_schedule.add_stage(name, stage);
        self
    }

    pub fn add_startup_stage_after<S: Stage>(
        &mut self,
        target: &'static str,
        name: &'static str,
        stage: S,
    ) -> &mut Self {
        self.app
            .startup_schedule
            .add_stage_after(target, name, stage);
        self
    }

    pub fn add_startup_stage_before<S: Stage>(
        &mut self,
        target: &'static str,
        name: &'static str,
        stage: S,
    ) -> &mut Self {
        self.app
            .startup_schedule
            .add_stage_before(target, name, stage);
        self
    }

    pub fn add_system<S, Params, IntoS>(&mut self, system: IntoS) -> &mut Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_system_to_stage(stage::UPDATE, system)
    }

    pub fn add_startup_system_to_stage<S, Params, IntoS>(
        &mut self,
        stage_name: &'static str,
        system: IntoS,
    ) -> &mut Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        let stage = self
            .app
            .startup_schedule
            .get_stage_mut::<SystemStage>(stage_name)
            .unwrap_or_else(|| {
                panic!(
                    "Startup stage '{}' does not exist or is not a SystemStage",
                    stage_name
                )
            });
        stage.add_system(system);
        self
    }

    pub fn add_startup_system<S, Params, IntoS>(&mut self, system: IntoS) -> &mut Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.add_startup_system_to_stage(startup_stage::STARTUP, system)
    }

    pub fn add_default_stages(&mut self) -> &mut Self {
        self.add_startup_stage(startup_stage::PRE_STARTUP, SystemStage::parallel())
            .add_startup_stage(startup_stage::STARTUP, SystemStage::parallel())
            .add_startup_stage(startup_stage::POST_STARTUP, SystemStage::parallel())
            .add_stage(stage::FIRST, SystemStage::parallel())
            .add_stage(stage::PRE_EVENT, SystemStage::parallel())
            .add_stage(stage::EVENT, SystemStage::parallel())
            .add_stage(stage::PRE_UPDATE, SystemStage::parallel())
            .add_stage(stage::UPDATE, SystemStage::parallel())
            .add_stage(stage::POST_UPDATE, SystemStage::parallel())
            .add_stage(stage::LAST, SystemStage::parallel())
    }

    pub fn add_system_to_stage<S, Params, IntoS>(
        &mut self,
        stage_name: &'static str,
        system: IntoS,
    ) -> &mut Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        let stage = self
            .app
            .schedule
            .get_stage_mut::<SystemStage>(stage_name)
            .unwrap_or_else(|| {
                panic!(
                    "Stage '{}' does not exist or is not a SystemStage",
                    stage_name
                )
            });
        stage.add_system(system);
        self
    }

    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.add_resource(Events::<T>::default())
            .add_system_to_stage(stage::EVENT, Events::<T>::update_system)
    }

    fn state_stage_name<T: Any>() -> String {
        format!("state({})", std::any::type_name::<T>())
    }

    pub fn add_state<T: Clone + Eq + Hash + Resource>(&mut self, initial: T) -> &mut Self {
        self.add_resource(State::new(initial));
        self.app.schedule.add_stage_after(
            stage::UPDATE,
            &Self::state_stage_name::<T>(),
            StateStage::<T>::default(),
        );
        self
    }

    pub fn on_state_update<T: Clone + Eq + Hash + Resource, S: Stage>(
        &mut self,
        value: T,
        stage: S,
    ) -> &mut Self {
        let state_stage = self
            .app
            .schedule
            .get_stage_mut::<StateStage<T>>(&Self::state_stage_name::<T>())
            .expect("State does not exist. Try calling app.add_state()");
        state_stage.set_update(value, stage);
        self
    }

    pub fn on_state_enter<T: Clone + Eq + Hash + Resource, S: Stage>(
        &mut self,
        value: T,
        stage: S,
    ) -> &mut Self {
        let state_stage = self
            .app
            .schedule
            .get_stage_mut::<StateStage<T>>(&Self::state_stage_name::<T>())
            .expect("State does not exist. Try calling app.add_state()");
        state_stage.set_enter(value, stage);
        self
    }

    pub fn on_state_exit<T: Clone + Eq + Hash + Resource, S: Stage>(
        &mut self,
        value: T,
        stage: S,
    ) -> &mut Self {
        let state_stage = self
            .app
            .schedule
            .get_stage_mut::<StateStage<T>>(&Self::state_stage_name::<T>())
            .expect("State does not exist. Try calling app.add_state()");
        state_stage.set_exit(value, stage);
        self
    }

    /// Adds a resource to the current [App] and overwrites any resource previously added of the same type.
    pub fn add_resource<T>(&mut self, resource: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.app.resources.insert(resource);
        self
    }

    pub fn add_thread_local_resource<T>(&mut self, resource: T) -> &mut Self
    where
        T: 'static,
    {
        self.app.resources.insert_thread_local(resource);
        self
    }

    pub fn init_resource<R>(&mut self) -> &mut Self
    where
        R: FromResources + Send + Sync + 'static,
    {
        let resource = R::from_resources(&self.app.resources);
        self.app.resources.insert(resource);

        self
    }

    pub fn init_thread_local_resource<R>(&mut self) -> &mut Self
    where
        R: FromResources + 'static,
    {
        let resource = R::from_resources(&self.app.resources);
        self.app.resources.insert_thread_local(resource);

        self
    }

    pub fn set_runner(&mut self, run_fn: impl Fn(App) + 'static) -> &mut Self {
        self.app.runner = Box::new(run_fn);
        self
    }

    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: Plugin,
    {
        debug!("added plugin: {}", plugin.name());
        plugin.build(self);
        self
    }

    pub fn add_plugins<T: PluginGroup>(&mut self, mut group: T) -> &mut Self {
        let mut plugin_group_builder = PluginGroupBuilder::default();
        group.build(&mut plugin_group_builder);
        plugin_group_builder.finish(self);
        self
    }

    pub fn add_plugins_with<T, F>(&mut self, mut group: T, func: F) -> &mut Self
    where
        T: PluginGroup,
        F: FnOnce(&mut PluginGroupBuilder) -> &mut PluginGroupBuilder,
    {
        let mut plugin_group_builder = PluginGroupBuilder::default();
        group.build(&mut plugin_group_builder);
        func(&mut plugin_group_builder);
        plugin_group_builder.finish(self);
        self
    }
}
