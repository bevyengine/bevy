use crate::{
    app::{App, AppExit},
    event::Events,
    plugin::Plugin,
    stage, startup_stage, PluginGroup, PluginGroupBuilder,
};
use bevy_ecs::{
    clear_trackers_system, FromResources, IntoSystem, Resource, Resources, RunOnce, Schedule,
    Stage, StateStage, System, SystemStage, World,
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
            .add_system_to_stage(stage::LAST, clear_trackers_system.system());
        app_builder
    }
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
        self.app
            .schedule
            .stage(stage::STARTUP, |schedule: &mut Schedule| {
                schedule.add_stage(name, stage)
            });
        self
    }

    pub fn add_startup_stage_after<S: Stage>(
        &mut self,
        target: &'static str,
        name: &'static str,
        stage: S,
    ) -> &mut Self {
        self.app
            .schedule
            .stage(stage::STARTUP, |schedule: &mut Schedule| {
                schedule.add_stage_after(target, name, stage)
            });
        self
    }

    pub fn add_startup_stage_before<S: Stage>(
        &mut self,
        target: &'static str,
        name: &'static str,
        stage: S,
    ) -> &mut Self {
        self.app
            .schedule
            .stage(stage::STARTUP, |schedule: &mut Schedule| {
                schedule.add_stage_before(target, name, stage)
            });
        self
    }

    pub fn stage<T: Stage, F: FnOnce(&mut T) -> &mut T>(
        &mut self,
        name: &str,
        func: F,
    ) -> &mut Self {
        self.app.schedule.stage(name, func);
        self
    }

    /// Adds a system that is run for every frame
    ///
    /// Systems are the main building block in bevy ECS model. You can define
    /// normal rust functions, and call `.system()` to make them be bevy systems.
    ///
    /// System functions can have parameters, through with one can query and
    /// mutate bevy ECS states. See bevy book for extra information.
    ///
    /// Systems are run in parallel, and the execution order is not deterministic.
    /// If you want more fine-grained control for order, see `add_system_to_stage`
    ///
    /// For adding a system that runs only at app startup, see `add_startup_system`
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    /// use bevy_ecs::prelude::*;
    ///
    /// fn my_system(_commands: &mut Commands) {
    ///     println!("My system, triggered once per frame");
    /// }
    ///
    /// App::build()
    ///     .add_system(my_system.system());
    /// ```
    pub fn add_system<S: System<In = (), Out = ()>>(&mut self, system: S) -> &mut Self {
        self.add_system_to_stage(stage::UPDATE, system)
    }

    pub fn on_state_enter<T: Clone + Resource, S: System<In = (), Out = ()>>(
        &mut self,
        stage: &str,
        state: T,
        system: S,
    ) -> &mut Self {
        self.stage(stage, |stage: &mut StateStage<T>| {
            stage.on_state_enter(state, system)
        })
    }

    pub fn on_state_update<T: Clone + Resource, S: System<In = (), Out = ()>>(
        &mut self,
        stage: &str,
        state: T,
        system: S,
    ) -> &mut Self {
        self.stage(stage, |stage: &mut StateStage<T>| {
            stage.on_state_update(state, system)
        })
    }

    pub fn on_state_exit<T: Clone + Resource, S: System<In = (), Out = ()>>(
        &mut self,
        stage: &str,
        state: T,
        system: S,
    ) -> &mut Self {
        self.stage(stage, |stage: &mut StateStage<T>| {
            stage.on_state_exit(state, system)
        })
    }

    pub fn add_startup_system_to_stage<S: System<In = (), Out = ()>>(
        &mut self,
        stage_name: &'static str,
        system: S,
    ) -> &mut Self {
        self.app
            .schedule
            .stage(stage::STARTUP, |schedule: &mut Schedule| {
                schedule.add_system_to_stage(stage_name, system)
            });
        self
    }

    /// Adds a system that is run once at application startup
    ///
    /// Startup systems run exactly once BEFORE all other systems. These are generally used for
    /// app initialization code (ex: adding entities and resources)
    ///
    /// For adding a system that runs for every frame, see `add_system`
    /// For adding a system to specific stage, see `add_system_to_stage`
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    /// use bevy_ecs::prelude::*;
    ///
    /// fn my_startup_system(_commands: &mut Commands) {
    ///     println!("My startup system");
    /// }
    ///
    /// App::build()
    ///     .add_startup_system(my_startup_system.system());
    /// ```
    pub fn add_startup_system<S: System<In = (), Out = ()>>(&mut self, system: S) -> &mut Self {
        self.add_startup_system_to_stage(startup_stage::STARTUP, system)
    }

    pub fn add_default_stages(&mut self) -> &mut Self {
        self.add_stage(
            stage::STARTUP,
            Schedule::default()
                .with_run_criteria(RunOnce::default())
                .with_stage(startup_stage::PRE_STARTUP, SystemStage::parallel())
                .with_stage(startup_stage::STARTUP, SystemStage::parallel())
                .with_stage(startup_stage::POST_STARTUP, SystemStage::parallel()),
        )
        .add_stage(stage::FIRST, SystemStage::parallel())
        .add_stage(stage::PRE_EVENT, SystemStage::parallel())
        .add_stage(stage::EVENT, SystemStage::parallel())
        .add_stage(stage::PRE_UPDATE, SystemStage::parallel())
        .add_stage(stage::UPDATE, SystemStage::parallel())
        .add_stage(stage::POST_UPDATE, SystemStage::parallel())
        .add_stage(stage::LAST, SystemStage::parallel())
    }

    pub fn add_system_to_stage<S: System<In = (), Out = ()>>(
        &mut self,
        stage_name: &'static str,
        system: S,
    ) -> &mut Self {
        self.app.schedule.add_system_to_stage(stage_name, system);
        self
    }

    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.insert_resource(Events::<T>::default())
            .add_system_to_stage(stage::EVENT, Events::<T>::update_system.system())
    }

    /// Inserts a resource to the current [App] and overwrites any resource previously added of the same type.
    ///
    /// A resource in bevy represents globally unique data. The resources must be added to bevy application
    /// before using them. This happens with `insert_resource`
    ///
    /// For adding a main-thread only accessible resource, see `insert_thread_local_resource`
    ///
    /// See also `init_resource` for resources that implement `Default` or `FromResources`
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    ///
    /// struct State {
    ///     counter: usize,
    /// }
    ///
    /// App::build()
    ///    .insert_resource(State { counter: 0 });
    /// ```
    pub fn insert_resource<T>(&mut self, resource: T) -> &mut Self
    where
        T: Send + Sync + 'static,
    {
        self.app.resources.insert(resource);
        self
    }

    /// Inserts a main thread local resource to the app
    ///
    /// Usually developers want to use `insert_resource`, but there are some special cases when a resource
    /// must be main-thread local.
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    ///
    /// #[derive(Default)]
    /// struct State {
    ///     counter: usize,
    /// }
    ///
    /// App::build()
    ///     .insert_thread_local_resource(State { counter: 0 });
    /// ```
    pub fn insert_thread_local_resource<T>(&mut self, resource: T) -> &mut Self
    where
        T: 'static,
    {
        self.app.resources.insert_thread_local(resource);
        self
    }

    /// Init a resource to the current [App] and overwrites any resource previously added of the same type.
    ///
    /// Adds a resource that implements `Default` or `FromResources` trait
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    ///
    /// struct State {
    ///     counter: usize,
    /// }
    ///
    /// impl Default for State {
    ///     fn default() -> State {
    ///         State {
    ///             counter: 100
    ///         }
    ///     }
    /// }
    ///
    /// App::build()
    ///     .init_resource::<State>();
    /// ```
    pub fn init_resource<R>(&mut self) -> &mut Self
    where
        R: FromResources + Send + Sync + 'static,
    {
        // PERF: We could avoid double hashing here, since the `from_resources` call is guaranteed not to
        // modify the map. However, we would need to be borrowing resources both mutably and immutably,
        // so we would need to be extremely certain this is correct
        if !self.resources().contains::<R>() {
            let resource = R::from_resources(&self.resources());
            self.insert_resource(resource);
        }

        self
    }

    pub fn init_thread_local_resource<R>(&mut self) -> &mut Self
    where
        R: FromResources + 'static,
    {
        // See perf comment in init_resource
        if self.app.resources.get_thread_local::<R>().is_none() {
            let resource = R::from_resources(&self.app.resources);
            self.app.resources.insert_thread_local(resource);
        }

        self
    }

    /// Sets the main runner loop function for bevy application
    ///
    /// Usually the main loop is handled by bevy integrated plugins (`winit`), but
    /// in some cases one wants to implement an own main loop.
    ///
    /// This method sets the main loop function. Overwrites previous runner.
    ///
    /// You should call `app.update()` in the runner to trigger the bevy ecs system.
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    ///
    /// fn my_runner(mut app: App) {
    ///     loop {
    ///         println!("In main loop");
    ///         app.update();
    ///     }
    /// }
    ///
    /// App::build()
    ///     .set_runner(my_runner);
    /// ```
    pub fn set_runner(&mut self, run_fn: impl Fn(App) + 'static) -> &mut Self {
        self.app.runner = Box::new(run_fn);
        self
    }

    /// Adds a single plugin
    ///
    /// One of Bevy's core principles is modularity. All bevy engines are implemented
    /// as plugins. This includes internal features like the renderer.
    ///
    /// Bevy also provides a few sets of default plugins. See `add_plugins`
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    ///
    /// App::build().add_plugin(bevy_log::LogPlugin::default());
    /// ```
    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: Plugin,
    {
        debug!("added plugin: {}", plugin.name());
        plugin.build(self);
        self
    }

    /// Adds a group of plugins
    ///
    /// Bevy plugins can be grouped into a set of plugins. By default
    /// bevy provides a few lists of plugins that can be used to kickstart
    /// the development.
    ///
    /// Current plugins offered are `DefaultPlugins` and `MinimalPlugins`
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    ///
    /// App::build();
    ///     //.add_plugins(MinimalPlugins)
    /// ```
    pub fn add_plugins<T: PluginGroup>(&mut self, mut group: T) -> &mut Self {
        let mut plugin_group_builder = PluginGroupBuilder::default();
        group.build(&mut plugin_group_builder);
        plugin_group_builder.finish(self);
        self
    }

    /// Adds a group of plugins with an initializer method
    ///
    /// Can be used to add a group of plugins, where the group is modified
    /// before insertion into Bevy application. For example, you can add
    /// extra plugins at a specific place in the plugin group.
    ///
    /// ## Example
    /// ```
    /// use bevy_app::prelude::*;
    ///
    /// App::build();
    ///     // .add_plugins_with(DefaultPlugins, |group| {
    ///            // group.add_before::<bevy::asset::AssetPlugin, _>(MyOwnPlugin)
    ///        // })
    /// ```
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
