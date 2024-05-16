use crate::{First, Main, MainSchedulePlugin, Plugin, Plugins, PluginsState, SubApp, SubApps};
pub use bevy_derive::AppLabel;
use bevy_ecs::{
    event::{event_update_system, ManualEventReader},
    intern::Interned,
    prelude::*,
    schedule::{ScheduleBuildSettings, ScheduleLabel},
    system::SystemId,
};
#[cfg(feature = "bevy_state")]
use bevy_state::{prelude::*, state::FreelyMutableState};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::{tracing::debug, HashMap};
use std::{fmt::Debug, process::{ExitCode, Termination}};
use std::{
    num::NonZeroU8,
};
use thiserror::Error;

bevy_ecs::define_label!(
    /// A strongly-typed class of labels used to identify an [`App`].
    AppLabel,
    APP_LABEL_INTERNER
);

pub use bevy_ecs::label::DynEq;
use crate::plugin_registry::{PluginState};

/// A shorthand for `Interned<dyn AppLabel>`.
pub type InternedAppLabel = Interned<dyn AppLabel>;

#[derive(Debug, Error)]
pub(crate) enum AppError {
    #[error("duplicate plugin {plugin_name:?}")]
    DuplicatePlugin { plugin_name: String },
}

#[allow(clippy::needless_doctest_main)]
/// [`App`] is the primary API for writing user applications. It automates the setup of a
/// [standard lifecycle](Main) and provides interface glue for [plugins](`Plugin`).
///
/// A single [`App`] can contain multiple [`SubApp`] instances, but [`App`] methods only affect
/// the "main" one. To access a particular [`SubApp`], use [`get_sub_app`](App::get_sub_app)
/// or [`get_sub_app_mut`](App::get_sub_app_mut).
///
///
/// # Examples
///
/// Here is a simple "Hello World" Bevy app:
///
/// ```
/// # use bevy_app::prelude::*;
/// # use bevy_ecs::prelude::*;
/// #
/// fn main() {
///    App::new()
///        .add_systems(Update, hello_world_system)
///        .run();
/// }
///
/// fn hello_world_system() {
///    println!("hello world");
/// }
/// ```
pub struct App {
    pub(crate) sub_apps: SubApps,
    /// The function that will manage the app's lifecycle.
    ///
    /// Bevy provides the [`WinitPlugin`] and [`ScheduleRunnerPlugin`] for windowed and headless
    /// applications, respectively.
    ///
    /// [`WinitPlugin`]: https://docs.rs/bevy/latest/bevy/winit/struct.WinitPlugin.html
    /// [`ScheduleRunnerPlugin`]: https://docs.rs/bevy/latest/bevy/app/struct.ScheduleRunnerPlugin.html
    pub(crate) runner: RunnerFn,
}

impl Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "App {{ sub_apps: ")?;
        f.debug_map()
            .entries(self.sub_apps.sub_apps.iter())
            .finish()?;
        write!(f, "}}")
    }
}

impl Default for App {
    fn default() -> Self {
        let mut app = App::empty();
        app.sub_apps.main.update_schedule = Some(Main.intern());

        #[cfg(feature = "bevy_reflect")]
        app.init_resource::<AppTypeRegistry>();
        app.add_plugins(MainSchedulePlugin);
        app.add_systems(
            First,
            event_update_system
                .in_set(bevy_ecs::event::EventUpdates)
                .run_if(bevy_ecs::event::event_update_condition),
        );
        app.add_event::<AppExit>();

        app
    }
}

impl App {
    /// Creates a new [`App`] with some default structure to enable core engine features.
    /// This is the preferred constructor for most use cases.
    pub fn new() -> App {
        App::default()
    }

    /// Creates a new empty [`App`] with minimal default configuration.
    ///
    /// Use this constructor if you want to customize scheduling, exit handling, cleanup, etc.
    pub fn empty() -> App {
        Self {
            sub_apps: SubApps {
                main: SubApp::new(),
                sub_apps: HashMap::new(),
            },
            runner: Box::new(run_once),
        }
    }

    /// Runs the default schedules of all sub-apps (starting with the "main" app) once.
    pub fn update(&mut self) {
        if self.is_building_plugins() {
            panic!("App::update() was called while a plugin was building.");
        }

        self.sub_apps.update();
    }

    /// Runs the [`App`] by calling its [runner](Self::set_runner).
    ///
    /// This will (re)build the [`App`] first. For general usage, see the example on the item
    /// level documentation.
    ///
    /// # Caveats
    ///
    /// Calls to [`App::run()`] will never return on iOS and Web.
    ///
    /// Headless apps can generally expect this method to return control to the caller when
    /// it completes, but that is not the case for windowed apps. Windowed apps are typically
    /// driven by an event loop and some platforms expect the program to terminate when the
    /// event loop ends.
    ///
    /// By default, *Bevy* uses the `winit` crate for window creation.
    ///
    /// # Panics
    ///
    /// Panics if not all plugins have been built.
    pub fn run(&mut self) -> AppExit {
        #[cfg(feature = "trace")]
        let _bevy_app_run_span = info_span!("bevy_app").entered();
        if self.is_building_plugins() {
            panic!("App::run() was called while a plugin was building.");
        }

        let runner = std::mem::replace(&mut self.runner, Box::new(run_once));
        let app = std::mem::replace(self, App::empty());
        (runner)(app)
    }

    /// Sets the function that will be called when the app is run.
    ///
    /// The runner function `f` is called only once by [`App::run`]. If the
    /// presence of a main loop in the app is desired, it is the responsibility of the runner
    /// function to provide it.
    ///
    /// The runner function is usually not set manually, but by Bevy integrated plugins
    /// (e.g. `WinitPlugin`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// #
    /// fn my_runner(mut app: App) -> AppExit {
    ///     loop {
    ///         println!("In main loop");
    ///         app.update();
    ///         if let Some(exit) = app.should_exit() {
    ///             return exit;
    ///         }
    ///     }
    /// }
    ///
    /// App::new()
    ///     .set_runner(my_runner);
    /// ```
    pub fn set_runner(&mut self, f: impl FnOnce(App) -> AppExit + 'static) -> &mut Self {
        self.runner = Box::new(f);
        self
    }

    /// Returns the state of all plugins. This is usually called by the event loop, but can be
    /// useful for situations where you want to use [`App::update`].
    #[inline]
    pub fn update_plugins(&mut self) {
        println!("Updating main plugins");
        self.main_mut().update_plugins();

        // overall state is the earliest state of any sub-app
        self.sub_apps.iter_mut().skip(1).for_each(|s| {
            println!("Updating subapp plugins");
            s.update_plugins();
        });
    }

    /// Returns the state of all plugins. This is usually called by the event loop, but can be
    /// useful for situations where you want to use [`App::update`].
    #[inline]
    pub fn plugins_state(&self) -> PluginsState {
        let mut overall_plugins_state = self.main().plugins_state();

        // overall state is the earliest state of any sub-app
        self.sub_apps.iter().skip(1).for_each(|s| {
            if s.plugins_state() != PluginsState::None {
                overall_plugins_state = overall_plugins_state.min(s.plugins_state());
            }
        });

        overall_plugins_state
    }

    /// Returns `true` if any of the sub-apps are building plugins.
    pub(crate) fn is_building_plugins(&self) -> bool {
        self.sub_apps.iter().any(|s| s.is_building_plugins())
    }

    #[cfg(feature = "bevy_state")]
    /// Initializes a [`State`] with standard starting values.
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`], [`OnTransition`] and [`OnExit`] schedules.
    /// These schedules are triggered before [`Update`](crate::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`] [`Condition`].
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](`bevy_ecs::schedule::StateTransition`) schedule manually.
    pub fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self {
        self.main_mut().init_state::<S>();
        self
    }

    #[cfg(feature = "bevy_state")]
    /// Inserts a specific [`State`] to the current [`App`] and overrides any [`State`] previously
    /// added of the same type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`], [`OnTransition`] and [`OnExit`] schedules.
    /// These schedules are triggered before [`Update`](crate::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`] [`Condition`].
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](`bevy_ecs::schedule::StateTransition`) schedule manually.
    pub fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        self.main_mut().insert_state::<S>(state);
        self
    }

    #[cfg(feature = "bevy_state")]
    /// Sets up a type implementing [`ComputedStates`].
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    pub fn add_computed_state<S: ComputedStates>(&mut self) -> &mut Self {
        self.main_mut().add_computed_state::<S>();
        self
    }

    #[cfg(feature = "bevy_state")]
    /// Sets up a type implementing [`SubStates`].
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    pub fn add_sub_state<S: SubStates>(&mut self) -> &mut Self {
        self.main_mut().add_sub_state::<S>();
        self
    }

    /// Adds one or more systems to the given schedule in this app's [`Schedules`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut app = App::new();
    /// # fn system_a() {}
    /// # fn system_b() {}
    /// # fn system_c() {}
    /// # fn should_run() -> bool { true }
    /// #
    /// app.add_systems(Update, (system_a, system_b, system_c));
    /// app.add_systems(Update, (system_a, system_b).run_if(should_run));
    /// ```
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoSystemConfigs<M>,
    ) -> &mut Self {
        self.main_mut().add_systems(schedule, systems);
        self
    }

    /// Registers a system and returns a [`SystemId`] so it can later be called by [`World::run_system`].
    ///
    /// It's possible to register the same systems more than once, they'll be stored separately.
    ///
    /// This is different from adding systems to a [`Schedule`] with [`App::add_systems`],
    /// because the [`SystemId`] that is returned can be used anywhere in the [`World`] to run the associated system.
    /// This allows for running systems in a push-based fashion.
    /// Using a [`Schedule`] is still preferred for most cases
    /// due to its better performance and ability to run non-conflicting systems simultaneously.
    pub fn register_system<I: 'static, O: 'static, M, S: IntoSystem<I, O, M> + 'static>(
        &mut self,
        system: S,
    ) -> SystemId<I, O> {
        self.main_mut().register_system(system)
    }

    /// Configures a collection of system sets in the provided schedule, adding any sets that do not exist.
    #[track_caller]
    pub fn configure_sets(
        &mut self,
        schedule: impl ScheduleLabel,
        sets: impl IntoSystemSetConfigs,
    ) -> &mut Self {
        self.main_mut().configure_sets(schedule, sets);
        self
    }

    /// Initializes `T` event handling by inserting an event queue resource ([`Events::<T>`])
    /// and scheduling an [`event_update_system`] in [`First`](crate::First).
    ///
    /// See [`Events`] for information on how to define events.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Event)]
    /// # struct MyEvent;
    /// # let mut app = App::new();
    /// #
    /// app.add_event::<MyEvent>();
    /// ```
    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Event,
    {
        self.main_mut().add_event::<T>();
        self
    }

    /// Inserts the [`Resource`] into the app, overwriting any existing resource of the same type.
    ///
    /// There is also an [`init_resource`](Self::init_resource) for resources that have
    /// [`Default`] or [`FromWorld`] implementations.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Resource)]
    /// struct MyCounter {
    ///     counter: usize,
    /// }
    ///
    /// App::new()
    ///    .insert_resource(MyCounter { counter: 0 });
    /// ```
    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.main_mut().insert_resource(resource);
        self
    }

    /// Inserts the [`Resource`], initialized with its default value, into the app,
    /// if there is no existing instance of `R`.
    ///
    /// `R` must implement [`FromWorld`].
    /// If `R` implements [`Default`], [`FromWorld`] will be automatically implemented and
    /// initialize the [`Resource`] with [`Default::default`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Resource)]
    /// struct MyCounter {
    ///     counter: usize,
    /// }
    ///
    /// impl Default for MyCounter {
    ///     fn default() -> MyCounter {
    ///         MyCounter {
    ///             counter: 100
    ///         }
    ///     }
    /// }
    ///
    /// App::new()
    ///     .init_resource::<MyCounter>();
    /// ```
    pub fn init_resource<R: Resource + FromWorld>(&mut self) -> &mut Self {
        self.main_mut().init_resource::<R>();
        self
    }

    /// Inserts the [`!Send`](Send) resource into the app, overwriting any existing resource
    /// of the same type.
    ///
    /// There is also an [`init_non_send_resource`](Self::init_non_send_resource) for
    /// resources that implement [`Default`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// struct MyCounter {
    ///     counter: usize,
    /// }
    ///
    /// App::new()
    ///     .insert_non_send_resource(MyCounter { counter: 0 });
    /// ```
    pub fn insert_non_send_resource<R: 'static>(&mut self, resource: R) -> &mut Self {
        self.world_mut().insert_non_send_resource(resource);
        self
    }

    /// Inserts the [`!Send`](Send) resource into the app, initialized with its default value,
    /// if there is no existing instance of `R`.
    pub fn init_non_send_resource<R: 'static + Default>(&mut self) -> &mut Self {
        self.world_mut().init_non_send_resource::<R>();
        self
    }

    pub(crate) fn add_boxed_plugin(
        &mut self,
        plugin: Box<dyn Plugin>,
    ) -> Result<&mut Self, AppError> {
        debug!("added plugin: {}", plugin.name());
        if plugin.is_unique()
            && !self
                .main_mut()
                .plugin_names
                .insert(plugin.name().to_string())
        {
            Err(AppError::DuplicatePlugin {
                plugin_name: plugin.name().to_string(),
            })?;
        }

        let mut plugin_registry = self.main_mut().take_plugin_registry();

        plugin_registry.add(plugin);

        plugin_registry.update(self);

        self.main_mut().insert_plugin_registry(plugin_registry);

        Ok(self)
    }

    /// Returns `true` if the [`Plugin`] has already been added.
    pub fn is_plugin_added<T>(&self) -> bool
    where
        T: Plugin,
    {
        self.main().is_plugin_added::<T>()
    }

    /// Returns a vector of references to all plugins of type `T` that have been added.
    ///
    /// This can be used to read the settings of any existing plugins.
    /// This vector will be empty if no plugins of that type have been added.
    /// If multiple copies of the same plugin are added to the [`App`], they will be listed in insertion order in this vector.
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # #[derive(Default)]
    /// # struct ImagePlugin {
    /// #    default_sampler: bool,
    /// # }
    /// # impl Plugin for ImagePlugin {
    /// #    fn build(&self, app: &mut App) {}
    /// # }
    /// # let mut app = App::new();
    /// # app.add_plugins(ImagePlugin::default());
    /// let default_sampler = app.get_added_plugins::<ImagePlugin>()[0].default_sampler;
    /// ```
    pub fn get_added_plugins<T>(&self) -> Vec<&T>
    where
        T: Plugin,
    {
        self.main().get_added_plugins::<T>()
    }

    /// Installs a [`Plugin`] collection.
    ///
    /// Bevy prioritizes modularity as a core principle. **All** engine features are implemented
    /// as plugins, even the complex ones like rendering.
    ///
    /// [`Plugin`]s can be grouped into a set by using a [`PluginGroup`].
    ///
    /// There are built-in [`PluginGroup`]s that provide core engine functionality.
    /// The [`PluginGroup`]s available by default are `DefaultPlugins` and `MinimalPlugins`.
    ///
    /// To customize the plugins in the group (reorder, disable a plugin, add a new plugin
    /// before / after another plugin), call [`build()`](super::PluginGroup::build) on the group,
    /// which will convert it to a [`PluginGroupBuilder`](crate::PluginGroupBuilder).
    ///
    /// You can also specify a group of [`Plugin`]s by using a tuple over [`Plugin`]s and
    /// [`PluginGroup`]s. See [`Plugins`] for more details.
    ///
    /// ## Examples
    /// ```
    /// # use bevy_app::{prelude::*, PluginGroupBuilder, NoopPluginGroup as MinimalPlugins};
    /// #
    /// # // Dummies created to avoid using `bevy_log`,
    /// # // which pulls in too many dependencies and breaks rust-analyzer
    /// # pub struct LogPlugin;
    /// # impl Plugin for LogPlugin {
    /// #     fn build(&self, app: &mut App) {}
    /// # }
    /// App::new()
    ///     .add_plugins(MinimalPlugins);
    /// App::new()
    ///     .add_plugins((MinimalPlugins, LogPlugin));
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if one of the plugins had already been added to the application.
    ///
    /// [`PluginGroup`]:super::PluginGroup
    #[track_caller]
    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        if self.plugins_state() >= PluginsState::Finalizing {
            panic!(
                "Plugins cannot be added after some are already being finalized."
            );
        }
        plugins.add_to_app(self);

        self
    }
    //
    // pub fn build_plugins(&mut self) -> Result<&mut Self, AppError>  {
    //     let plugins = std::mem::take(&mut self.main_mut().plugin_registry);
    //
    //     self.main_mut().plugin_build_depth += 1;
    //     for plugin in &plugins {
    //         let result = catch_unwind(AssertUnwindSafe(|| plugin.build(self)));
    //         if let Err(payload) = result {
    //             resume_unwind(payload);
    //         }
    //     }
    //     self.main_mut().plugin_build_depth -= 1;
    //
    //     self.main_mut().plugin_registry = plugins;
    //
    //     Ok(self)
    // }

    /// Registers the type `T` in the [`TypeRegistry`](bevy_reflect::TypeRegistry) resource,
    /// adding reflect data as specified in the [`Reflect`](bevy_reflect::Reflect) derive:
    /// ```ignore (No serde "derive" feature)
    /// #[derive(Component, Serialize, Deserialize, Reflect)]
    /// #[reflect(Component, Serialize, Deserialize)] // will register ReflectComponent, ReflectSerialize, ReflectDeserialize
    /// ```
    ///
    /// See [`bevy_reflect::TypeRegistry::register`] for more information.
    #[cfg(feature = "bevy_reflect")]
    pub fn register_type<T: bevy_reflect::GetTypeRegistration>(&mut self) -> &mut Self {
        self.main_mut().register_type::<T>();
        self
    }

    /// Associates type data `D` with type `T` in the [`TypeRegistry`](bevy_reflect::TypeRegistry) resource.
    ///
    /// Most of the time [`register_type`](Self::register_type) can be used instead to register a
    /// type you derived [`Reflect`](bevy_reflect::Reflect) for. However, in cases where you want to
    /// add a piece of type data that was not included in the list of `#[reflect(...)]` type data in
    /// the derive, or where the type is generic and cannot register e.g. `ReflectSerialize`
    /// unconditionally without knowing the specific type parameters, this method can be used to
    /// insert additional type data.
    ///
    /// # Example
    /// ```
    /// use bevy_app::App;
    /// use bevy_reflect::{ReflectSerialize, ReflectDeserialize};
    ///
    /// App::new()
    ///     .register_type::<Option<String>>()
    ///     .register_type_data::<Option<String>, ReflectSerialize>()
    ///     .register_type_data::<Option<String>, ReflectDeserialize>();
    /// ```
    ///
    /// See [`bevy_reflect::TypeRegistry::register_type_data`].
    #[cfg(feature = "bevy_reflect")]
    pub fn register_type_data<
        T: bevy_reflect::Reflect + bevy_reflect::TypePath,
        D: bevy_reflect::TypeData + bevy_reflect::FromType<T>,
    >(
        &mut self,
    ) -> &mut Self {
        self.main_mut().register_type_data::<T, D>();
        self
    }

    /// Returns a reference to the [`World`].
    pub fn world(&self) -> &World {
        self.main().world()
    }

    /// Returns a mutable reference to the [`World`].
    pub fn world_mut(&mut self) -> &mut World {
        self.main_mut().world_mut()
    }

    /// Returns a reference to the main [`SubApp`].
    pub fn main(&self) -> &SubApp {
        &self.sub_apps.main
    }

    /// Returns a mutable reference to the main [`SubApp`].
    pub fn main_mut(&mut self) -> &mut SubApp {
        &mut self.sub_apps.main
    }

    /// Returns a reference to the [`SubApp`] with the given label.
    ///
    /// # Panics
    ///
    /// Panics if the [`SubApp`] doesn't exist.
    pub fn sub_app(&self, label: impl AppLabel) -> &SubApp {
        let str = label.intern();
        self.get_sub_app(label).unwrap_or_else(|| {
            panic!("No sub-app with label '{:?}' exists.", str);
        })
    }

    /// Returns a reference to the [`SubApp`] with the given label.
    ///
    /// # Panics
    ///
    /// Panics if the [`SubApp`] doesn't exist.
    pub fn sub_app_mut(&mut self, label: impl AppLabel) -> &mut SubApp {
        let str = label.intern();
        self.get_sub_app_mut(label).unwrap_or_else(|| {
            panic!("No sub-app with label '{:?}' exists.", str);
        })
    }

    /// Returns a reference to the [`SubApp`] with the given label, if it exists.
    pub fn get_sub_app(&self, label: impl AppLabel) -> Option<&SubApp> {
        self.sub_apps.sub_apps.get(&label.intern())
    }

    /// Returns a mutable reference to the [`SubApp`] with the given label, if it exists.
    pub fn get_sub_app_mut(&mut self, label: impl AppLabel) -> Option<&mut SubApp> {
        self.sub_apps.sub_apps.get_mut(&label.intern())
    }

    /// Inserts a [`SubApp`] with the given label.
    pub fn insert_sub_app(&mut self, label: impl AppLabel, sub_app: SubApp) {
        self.sub_apps.sub_apps.insert(label.intern(), sub_app);
    }

    /// Removes the [`SubApp`] with the given label, if it exists.
    pub fn remove_sub_app(&mut self, label: impl AppLabel) -> Option<SubApp> {
        self.sub_apps.sub_apps.remove(&label.intern())
    }

    /// Inserts a new `schedule` under the provided `label`, overwriting any existing
    /// schedule with the same label.
    pub fn add_schedule(&mut self, schedule: Schedule) -> &mut Self {
        self.main_mut().add_schedule(schedule);
        self
    }

    /// Initializes an empty `schedule` under the provided `label`, if it does not exist.
    ///
    /// See [`add_schedule`](Self::add_schedule) to insert an existing schedule.
    pub fn init_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        self.main_mut().init_schedule(label);
        self
    }

    /// Returns a reference to the [`Schedule`] with the provided `label` if it exists.
    pub fn get_schedule(&self, label: impl ScheduleLabel) -> Option<&Schedule> {
        self.main().get_schedule(label)
    }

    /// Returns a mutable reference to the [`Schedule`] with the provided `label` if it exists.
    pub fn get_schedule_mut(&mut self, label: impl ScheduleLabel) -> Option<&mut Schedule> {
        self.main_mut().get_schedule_mut(label)
    }

    /// Runs function `f` with the [`Schedule`] associated with `label`.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    pub fn edit_schedule(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnMut(&mut Schedule),
    ) -> &mut Self {
        self.main_mut().edit_schedule(label, f);
        self
    }

    /// Applies the provided [`ScheduleBuildSettings`] to all schedules.
    pub fn configure_schedules(
        &mut self,
        schedule_build_settings: ScheduleBuildSettings,
    ) -> &mut Self {
        self.main_mut().configure_schedules(schedule_build_settings);
        self
    }

    /// When doing [ambiguity checking](ScheduleBuildSettings) this
    /// ignores systems that are ambiguous on [`Component`] T.
    ///
    /// This settings only applies to the main world. To apply this to other worlds call the
    /// [corresponding method](World::allow_ambiguous_component) on World
    ///
    /// ## Example
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};
    /// # use bevy_utils::default;
    ///
    /// #[derive(Component)]
    /// struct A;
    ///
    /// // these systems are ambiguous on A
    /// fn system_1(_: Query<&mut A>) {}
    /// fn system_2(_: Query<&A>) {}
    ///
    /// let mut app = App::new();
    /// app.configure_schedules(ScheduleBuildSettings {
    ///   ambiguity_detection: LogLevel::Error,
    ///   ..default()
    /// });
    ///
    /// app.add_systems(Update, ( system_1, system_2 ));
    /// app.allow_ambiguous_component::<A>();
    ///
    /// // running the app does not error.
    /// app.update();
    /// ```
    pub fn allow_ambiguous_component<T: Component>(&mut self) -> &mut Self {
        self.main_mut().allow_ambiguous_component::<T>();
        self
    }

    /// When doing [ambiguity checking](ScheduleBuildSettings) this
    /// ignores systems that are ambiguous on [`Resource`] T.
    ///
    /// This settings only applies to the main world. To apply this to other worlds call the
    /// [corresponding method](World::allow_ambiguous_resource) on World
    ///
    /// ## Example
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_ecs::schedule::{LogLevel, ScheduleBuildSettings};
    /// # use bevy_utils::default;
    ///
    /// #[derive(Resource)]
    /// struct R;
    ///
    /// // these systems are ambiguous on R
    /// fn system_1(_: ResMut<R>) {}
    /// fn system_2(_: Res<R>) {}
    ///
    /// let mut app = App::new();
    /// app.configure_schedules(ScheduleBuildSettings {
    ///   ambiguity_detection: LogLevel::Error,
    ///   ..default()
    /// });
    /// app.insert_resource(R);
    ///
    /// app.add_systems(Update, ( system_1, system_2 ));
    /// app.allow_ambiguous_resource::<R>();
    ///
    /// // running the app does not error.
    /// app.update();
    /// ```
    pub fn allow_ambiguous_resource<T: Resource>(&mut self) -> &mut Self {
        self.main_mut().allow_ambiguous_resource::<T>();
        self
    }

    /// Suppress warnings and errors that would result from systems in these sets having ambiguities
    /// (conflicting access but indeterminate order) with systems in `set`.
    ///
    /// When possible, do this directly in the `.add_systems(Update, a.ambiguous_with(b))` call.
    /// However, sometimes two independent plugins `A` and `B` are reported as ambiguous, which you
    /// can only suppress as the consumer of both.
    #[track_caller]
    pub fn ignore_ambiguity<M1, M2, S1, S2>(
        &mut self,
        schedule: impl ScheduleLabel,
        a: S1,
        b: S2,
    ) -> &mut Self
    where
        S1: IntoSystemSet<M1>,
        S2: IntoSystemSet<M2>,
    {
        self.main_mut().ignore_ambiguity(schedule, a, b);
        self
    }

    /// Attempts to determine if an [`AppExit`] was raised since the last update.
    ///
    /// Will attempt to return the first [`Error`](AppExit::Error) it encounters.
    /// This should be called after every [`update()`](App::update) otherwise you risk
    /// dropping possible [`AppExit`] events.
    pub fn should_exit(&self) -> Option<AppExit> {
        let mut reader = ManualEventReader::default();

        let events = self.world().get_resource::<Events<AppExit>>()?;
        let mut events = reader.read(events);

        if events.len() != 0 {
            return Some(
                events
                    .find(|exit| exit.is_error())
                    .cloned()
                    .unwrap_or(AppExit::Success),
            );
        }

        None
    }
}

type RunnerFn = Box<dyn FnOnce(App) -> AppExit>;

fn run_once(mut app: App) -> AppExit {
    println!("running once");
    while app.plugins_state() != PluginsState::Done {
        app.update_plugins();

        #[cfg(not(target_arch = "wasm32"))]
        bevy_tasks::tick_global_task_pools_on_main_thread();
    }

    app.update();

    let mut exit_code_reader = ManualEventReader::default();
    if let Some(app_exit_events) = app.world().get_resource::<Events<AppExit>>() {
        if exit_code_reader
            .read(app_exit_events)
            .any(AppExit::is_error)
        {
            return AppExit::error();
        }
    }

    AppExit::Success
}

/// An event that indicates the [`App`] should exit. If one or more of these are present at the end of an update,
/// the [runner](App::set_runner) will end and ([maybe](App::run)) return control to the caller.
///
/// This event can be used to detect when an exit is requested. Make sure that systems listening
/// for this event run before the current update ends.
///
/// # Portability
/// This type is roughly meant to map to a standard definition of a process exit code (0 means success, not 0 means error). Due to portability concerns
/// (see [`ExitCode`](https://doc.rust-lang.org/std/process/struct.ExitCode.html) and [`process::exit`](https://doc.rust-lang.org/std/process/fn.exit.html#))
/// we only allow error codes between 1 and [255](u8::MAX).
#[derive(Event, Debug, Clone, Default, PartialEq, Eq)]
pub enum AppExit {
    /// [`App`] exited without any problems.
    #[default]
    Success,
    /// The [`App`] experienced an unhandleable error.
    /// Holds the exit code we expect our app to return.
    Error(NonZeroU8),
}

impl AppExit {
    /// Creates a [`AppExit::Error`] with a error code of 1.
    #[must_use]
    pub const fn error() -> Self {
        Self::Error(NonZeroU8::MIN)
    }

    /// Returns `true` if `self` is a [`AppExit::Success`].
    #[must_use]
    pub const fn is_success(&self) -> bool {
        matches!(self, AppExit::Success)
    }

    /// Returns `true` if `self` is a [`AppExit::Error`].
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, AppExit::Error(_))
    }

    /// Creates a [`AppExit`] from a code.
    ///
    /// When `code` is 0 a [`AppExit::Success`] is constructed otherwise a
    /// [`AppExit::Error`] is constructed.
    #[must_use]
    pub const fn from_code(code: u8) -> Self {
        match NonZeroU8::new(code) {
            Some(code) => Self::Error(code),
            None => Self::Success,
        }
    }
}

impl From<u8> for AppExit {
    #[must_use]
    fn from(value: u8) -> Self {
        Self::from_code(value)
    }
}

impl Termination for AppExit {
    fn report(self) -> std::process::ExitCode {
        match self {
            AppExit::Success => ExitCode::SUCCESS,
            // We leave logging an error to our users
            AppExit::Error(value) => ExitCode::from(value.get()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{marker::PhantomData, mem};

    use bevy_ecs::{schedule::ScheduleLabel, system::Commands};

    use crate::{App, AppExit, Plugin, PluginsState, Update};
    use crate::plugin_registry::PluginState;

    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, _app: &mut App) {}
    }
    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, _app: &mut App) {}
    }
    struct PluginC<T>(T);
    impl<T: Send + Sync + 'static> Plugin for PluginC<T> {
        fn build(&self, _app: &mut App) {}
    }
    struct PluginD;
    impl Plugin for PluginD {
        fn build(&self, _app: &mut App) {}
        fn is_unique(&self) -> bool {
            false
        }
    }

    struct PluginE;

    impl Plugin for PluginE {
        fn build(&self, _app: &mut App) {}

        fn finish(&self, app: &mut App) {
            if app.is_plugin_added::<PluginA>() {
                panic!("cannot run if PluginA is already registered");
            }
        }
    }

    #[test]
    fn simple() {
        fn hello_world_system() {
            println!("hello world");
        }

        println!("{:?}", std::thread::current());
        App::new()
               .add_systems(Update, hello_world_system)
               .run();
    }

    #[test]
    fn can_add_two_plugins() {
        App::new().add_plugins((PluginA, PluginB));
    }

    #[test]
    #[should_panic]
    fn cant_add_twice_the_same_plugin() {
        App::new().add_plugins((PluginA, PluginA));
    }

    #[test]
    fn can_add_twice_the_same_plugin_with_different_type_param() {
        App::new().add_plugins((PluginC(0), PluginC(true)));
    }

    #[test]
    fn can_add_twice_the_same_plugin_not_unique() {
        App::new().add_plugins((PluginD, PluginD));
    }

    #[test]
    #[should_panic]
    fn cant_call_app_run_from_plugin_build() {
        struct PluginRun;
        struct InnerPlugin;
        impl Plugin for InnerPlugin {
            fn build(&self, _: &mut App) {}
        }
        impl Plugin for PluginRun {
            fn build(&self, app: &mut App) {
                app.add_plugins(InnerPlugin).run();
            }
        }

        let mut app = App::new();

        app.add_plugins(PluginRun);

        assert_eq!(app.plugins_state(), PluginsState::Init);

        app.update_plugins();
    }

    #[derive(ScheduleLabel, Hash, Clone, PartialEq, Eq, Debug)]
    struct EnterMainMenu;

    fn bar(mut commands: Commands) {
        commands.spawn_empty();
    }

    fn foo(mut commands: Commands) {
        commands.spawn_empty();
    }

    #[test]
    fn add_systems_should_create_schedule_if_it_does_not_exist() {
        let mut app = App::new();
        app.add_systems(EnterMainMenu, (foo, bar));

        app.world_mut().run_schedule(EnterMainMenu);
        assert_eq!(app.world().entities().len(), 2);
    }

    #[test]
    #[should_panic]
    fn plugin_cannot_be_added_during_finish() {
        let mut app = App::new();
        app.add_plugins(PluginA);
        assert_eq!(app.plugins_state(), PluginsState::Init);

        app.update_plugins(); // Build
        assert_eq!(app.plugins_state(), PluginsState::Building);
        app.update_plugins(); //
        assert_eq!(app.plugins_state(), PluginsState::Finalizing);
        app.update_plugins();
        assert_eq!(app.plugins_state(), PluginsState::Done);

        app.add_plugins(PluginE);

        app.update_plugins();
        // app.finish();
    }

    #[test]
    fn test_derive_app_label() {
        use super::AppLabel;
        use crate::{self as bevy_app};

        #[derive(AppLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct UnitLabel;

        #[derive(AppLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct TupleLabel(u32, u32);

        #[derive(AppLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct StructLabel {
            a: u32,
            b: u32,
        }

        #[derive(AppLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct EmptyTupleLabel();

        #[derive(AppLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct EmptyStructLabel {}

        #[derive(AppLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        enum EnumLabel {
            #[default]
            Unit,
            Tuple(u32, u32),
            Struct {
                a: u32,
                b: u32,
            },
        }

        #[derive(AppLabel, Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
        struct GenericLabel<T>(PhantomData<T>);

        assert_eq!(UnitLabel.intern(), UnitLabel.intern());
        assert_eq!(EnumLabel::Unit.intern(), EnumLabel::Unit.intern());
        assert_ne!(UnitLabel.intern(), EnumLabel::Unit.intern());
        assert_ne!(UnitLabel.intern(), TupleLabel(0, 0).intern());
        assert_ne!(EnumLabel::Unit.intern(), EnumLabel::Tuple(0, 0).intern());

        assert_eq!(TupleLabel(0, 0).intern(), TupleLabel(0, 0).intern());
        assert_eq!(
            EnumLabel::Tuple(0, 0).intern(),
            EnumLabel::Tuple(0, 0).intern()
        );
        assert_ne!(TupleLabel(0, 0).intern(), TupleLabel(0, 1).intern());
        assert_ne!(
            EnumLabel::Tuple(0, 0).intern(),
            EnumLabel::Tuple(0, 1).intern()
        );
        assert_ne!(TupleLabel(0, 0).intern(), EnumLabel::Tuple(0, 0).intern());
        assert_ne!(
            TupleLabel(0, 0).intern(),
            StructLabel { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            EnumLabel::Tuple(0, 0).intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );

        assert_eq!(
            StructLabel { a: 0, b: 0 }.intern(),
            StructLabel { a: 0, b: 0 }.intern()
        );
        assert_eq!(
            EnumLabel::Struct { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            StructLabel { a: 0, b: 0 }.intern(),
            StructLabel { a: 0, b: 1 }.intern()
        );
        assert_ne!(
            EnumLabel::Struct { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 1 }.intern()
        );
        assert_ne!(
            StructLabel { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(
            StructLabel { a: 0, b: 0 }.intern(),
            EnumLabel::Struct { a: 0, b: 0 }.intern()
        );
        assert_ne!(StructLabel { a: 0, b: 0 }.intern(), UnitLabel.intern(),);
        assert_ne!(
            EnumLabel::Struct { a: 0, b: 0 }.intern(),
            EnumLabel::Unit.intern()
        );

        assert_eq!(
            GenericLabel::<u32>(PhantomData).intern(),
            GenericLabel::<u32>(PhantomData).intern()
        );
        assert_ne!(
            GenericLabel::<u32>(PhantomData).intern(),
            GenericLabel::<u64>(PhantomData).intern()
        );
    }

    /// Custom runners should be in charge of when `app::update` gets called as they may need to
    /// coordinate some state.
    /// bug: <https://github.com/bevyengine/bevy/issues/10385>
    /// fix: <https://github.com/bevyengine/bevy/pull/10389>
    #[test]
    fn regression_test_10385() {
        use super::{Res, Resource};
        use crate::PreUpdate;

        #[derive(Resource)]
        struct MyState {}

        fn my_runner(mut app: App) -> AppExit {
            let my_state = MyState {};
            app.update_plugins();
            app.world_mut().insert_resource(my_state);

            for _ in 0..5 {
                app.update();
            }

            AppExit::Success
        }

        fn my_system(_: Res<MyState>) {
            // access state during app update
        }

        // Should not panic due to missing resource
        App::new()
            .set_runner(my_runner)
            .add_systems(PreUpdate, my_system)
            .run();
    }

    #[test]
    fn app_exit_size() {
        // There wont be many of them so the size isn't a issue but
        // it's nice they're so small let's keep it that way.
        assert_eq!(mem::size_of::<AppExit>(), mem::size_of::<u8>());
    }
}
