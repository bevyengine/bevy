use crate::{First, Main, MainSchedulePlugin, Plugin, Plugins, StateTransition};
pub use bevy_derive::AppLabel;
use bevy_ecs::{
    prelude::*,
    schedule::{
        apply_state_transition, common_conditions::run_once as run_once_condition,
        run_enter_schedule, BoxedScheduleLabel, IntoSystemConfigs, IntoSystemSetConfigs,
        ScheduleBuildSettings, ScheduleLabel,
    },
};
use bevy_utils::{tracing::debug, HashMap, HashSet};
use std::{
    fmt::Debug,
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
};

#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

bevy_utils::define_label!(
    /// A strongly-typed class of labels used to identify an [`App`].
    AppLabel,
    /// A strongly-typed identifier for an [`AppLabel`].
    AppLabelId,
);

pub(crate) enum AppError {
    DuplicatePlugin { plugin_name: String },
}

#[allow(clippy::needless_doctest_main)]
/// A container of app logic and data.
///
/// Bundles together the necessary elements like [`World`] and [`Schedule`] to create
/// an ECS-based application. It also stores a pointer to a [runner function](Self::set_runner).
/// The runner is responsible for managing the application's event loop and applying the
/// [`Schedule`] to the [`World`] to drive application logic.
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
    /// The main ECS [`World`] of the [`App`].
    /// This stores and provides access to all the main data of the application.
    /// The systems of the [`App`] will run using this [`World`].
    /// If additional separate [`World`]-[`Schedule`] pairs are needed, you can use [`sub_app`](App::insert_sub_app)s.
    pub world: World,
    /// The [runner function](Self::set_runner) is primarily responsible for managing
    /// the application's event loop and advancing the [`Schedule`].
    /// Typically, it is not configured manually, but set by one of Bevy's built-in plugins.
    /// See `bevy::winit::WinitPlugin` and [`ScheduleRunnerPlugin`](crate::schedule_runner::ScheduleRunnerPlugin).
    pub runner: Box<dyn FnOnce(App) + Send>, // Send bound is required to make App Send
    /// The schedule that systems are added to by default.
    ///
    /// The schedule that runs the main loop of schedule execution.
    ///
    /// This is initially set to [`Main`].
    pub main_schedule_label: BoxedScheduleLabel,
    sub_apps: HashMap<AppLabelId, SubApp>,
    plugin_registry: Vec<Box<dyn Plugin>>,
    plugin_name_added: HashSet<String>,
    /// A private counter to prevent incorrect calls to `App::run()` from `Plugin::build()`
    building_plugin_depth: usize,
}

impl Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "App {{ sub_apps: ")?;
        f.debug_map()
            .entries(self.sub_apps.iter().map(|(k, v)| (k, v)))
            .finish()?;
        write!(f, "}}")
    }
}

/// A [`SubApp`] contains its own [`Schedule`] and [`World`] separate from the main [`App`].
/// This is useful for situations where data and data processing should be kept completely separate
/// from the main application. The primary use of this feature in bevy is to enable pipelined rendering.
///
/// # Example
///
/// ```rust
/// # use bevy_app::{App, AppLabel, SubApp, Main};
/// # use bevy_ecs::prelude::*;
/// # use bevy_ecs::schedule::ScheduleLabel;
///
/// #[derive(Resource, Default)]
/// struct Val(pub i32);
///
/// #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, AppLabel)]
/// struct ExampleApp;
///
/// let mut app = App::new();
///
/// // initialize the main app with a value of 0;
/// app.insert_resource(Val(10));
///
/// // create a app with a resource and a single schedule
/// let mut sub_app = App::empty();
/// // add an outer schedule that runs the main schedule
/// sub_app.insert_resource(Val(100));
///
/// // initialize main schedule
/// sub_app.add_systems(Main, |counter: Res<Val>| {
///     // since we assigned the value from the main world in extract
///     // we see that value instead of 100
///     assert_eq!(counter.0, 10);
/// });
///
/// // add the sub_app to the app
/// app.insert_sub_app(ExampleApp, SubApp::new(sub_app, |main_world, sub_app| {
///     // extract the value from the main app to the sub app
///     sub_app.world.resource_mut::<Val>().0 = main_world.resource::<Val>().0;
/// }));
///
/// // This will run the schedules once, since we're using the default runner
/// app.run();
/// ```
pub struct SubApp {
    /// The [`SubApp`]'s instance of [`App`]
    pub app: App,

    /// A function that allows access to both the main [`App`] [`World`] and the [`SubApp`]. This is
    /// useful for moving data between the sub app and the main app.
    extract: Box<dyn Fn(&mut World, &mut App) + Send>,
}

impl SubApp {
    /// Creates a new [`SubApp`].
    ///
    /// The provided function `extract` is normally called by the [`update`](App::update) method.
    /// After extract is called, the [`Schedule`] of the sub app is run. The [`World`]
    /// parameter represents the main app world, while the [`App`] parameter is just a mutable
    /// reference to the `SubApp` itself.
    pub fn new(app: App, extract: impl Fn(&mut World, &mut App) + Send + 'static) -> Self {
        Self {
            app,
            extract: Box::new(extract),
        }
    }

    /// Runs the [`SubApp`]'s default schedule.
    pub fn run(&mut self) {
        self.app.world.run_schedule(&*self.app.main_schedule_label);
        self.app.world.clear_trackers();
    }

    /// Extracts data from main world to this sub-app.
    pub fn extract(&mut self, main_world: &mut World) {
        (self.extract)(main_world, &mut self.app);
    }
}

impl Debug for SubApp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SubApp {{ app: ")?;
        f.debug_map()
            .entries(self.app.sub_apps.iter().map(|(k, v)| (k, v)))
            .finish()?;
        write!(f, "}}")
    }
}

impl Default for App {
    fn default() -> Self {
        let mut app = App::empty();
        #[cfg(feature = "bevy_reflect")]
        app.init_resource::<AppTypeRegistry>();

        app.add_plugins(MainSchedulePlugin);
        app.add_event::<AppExit>();

        #[cfg(feature = "bevy_ci_testing")]
        {
            crate::ci_testing::setup_app(&mut app);
        }

        app
    }
}

// Dummy plugin used to temporary hold the place in the plugin registry
struct PlaceholderPlugin;
impl Plugin for PlaceholderPlugin {
    fn build(&self, _app: &mut App) {}
}

impl App {
    /// Creates a new [`App`] with some default structure to enable core engine features.
    /// This is the preferred constructor for most use cases.
    pub fn new() -> App {
        App::default()
    }

    /// Creates a new empty [`App`] with minimal default configuration.
    ///
    /// This constructor should be used if you wish to provide custom scheduling, exit handling, cleanup, etc.
    pub fn empty() -> App {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        Self {
            world,
            runner: Box::new(run_once),
            sub_apps: HashMap::default(),
            plugin_registry: Vec::default(),
            plugin_name_added: Default::default(),
            main_schedule_label: Box::new(Main),
            building_plugin_depth: 0,
        }
    }

    /// Advances the execution of the [`Schedule`] by one cycle.
    ///
    /// This method also updates sub apps.
    /// See [`insert_sub_app`](Self::insert_sub_app) for more details.
    ///
    /// The schedule run by this method is determined by the [`main_schedule_label`](App) field.
    /// By default this is [`Main`].
    ///
    /// # Panics
    ///
    /// The active schedule of the app must be set before this method is called.
    pub fn update(&mut self) {
        #[cfg(feature = "trace")]
        let _bevy_update_span = info_span!("update").entered();
        {
            #[cfg(feature = "trace")]
            let _bevy_main_update_span = info_span!("main app").entered();
            self.world.run_schedule(&*self.main_schedule_label);
        }
        for (_label, sub_app) in &mut self.sub_apps {
            #[cfg(feature = "trace")]
            let _sub_app_span = info_span!("sub app", name = ?_label).entered();
            sub_app.extract(&mut self.world);
            sub_app.run();
        }

        self.world.clear_trackers();
    }

    /// Starts the application by calling the app's [runner function](Self::set_runner).
    ///
    /// Finalizes the [`App`] configuration. For general usage, see the example on the item
    /// level documentation.
    ///
    /// # `run()` might not return
    ///
    /// Calls to [`App::run()`] might never return.
    ///
    /// In simple and *headless* applications, one can expect that execution will
    /// proceed, normally, after calling [`run()`](App::run()) but this is not the case for
    /// windowed applications.
    ///
    /// Windowed apps are typically driven by an *event loop* or *message loop* and
    /// some window-manager APIs expect programs to terminate when their primary
    /// window is closed and that event loop terminates – behavior of processes that
    /// do not is often platform dependent or undocumented.
    ///
    /// By default, *Bevy* uses the `winit` crate for window creation. See
    /// [`WinitSettings::return_from_run`](https://docs.rs/bevy/latest/bevy/winit/struct.WinitSettings.html#structfield.return_from_run)
    /// for further discussion of this topic and for a mechanism to require that [`App::run()`]
    /// *does* return – albeit one that carries its own caveats and disclaimers.
    ///
    /// # Panics
    ///
    /// Panics if called from `Plugin::build()`, because it would prevent other plugins to properly build.
    pub fn run(&mut self) {
        #[cfg(feature = "trace")]
        let _bevy_app_run_span = info_span!("bevy_app").entered();

        let mut app = std::mem::replace(self, App::empty());
        if app.building_plugin_depth > 0 {
            panic!("App::run() was called from within Plugin::build(), which is not allowed.");
        }

        let runner = std::mem::replace(&mut app.runner, Box::new(run_once));
        (runner)(app);
    }

    /// Check that [`Plugin::ready`] of all plugins returns true. This is usually called by the
    /// event loop, but can be useful for situations where you want to use [`App::update`]
    pub fn ready(&self) -> bool {
        for plugin in &self.plugin_registry {
            if !plugin.ready(self) {
                return false;
            }
        }
        true
    }

    /// Run [`Plugin::finish`] for each plugin. This is usually called by the event loop once all
    /// plugins are [`App::ready`], but can be useful for situations where you want to use
    /// [`App::update`].
    pub fn finish(&mut self) {
        // temporarily remove the plugin registry to run each plugin's setup function on app.
        let plugin_registry = std::mem::take(&mut self.plugin_registry);
        for plugin in &plugin_registry {
            plugin.finish(self);
        }
        self.plugin_registry = plugin_registry;
    }

    /// Run [`Plugin::cleanup`] for each plugin. This is usually called by the event loop after
    /// [`App::finish`], but can be useful for situations where you want to use [`App::update`].
    pub fn cleanup(&mut self) {
        // temporarily remove the plugin registry to run each plugin's setup function on app.
        let plugin_registry = std::mem::take(&mut self.plugin_registry);
        for plugin in &plugin_registry {
            plugin.cleanup(self);
        }
        self.plugin_registry = plugin_registry;
    }

    /// Adds [`State<S>`] and [`NextState<S>`] resources, [`OnEnter`] and [`OnExit`] schedules
    /// for each state variant (if they don't already exist), an instance of [`apply_state_transition::<S>`] in
    /// [`StateTransition`] so that transitions happen before [`Update`](crate::Update) and
    /// a instance of [`run_enter_schedule::<S>`] in [`StateTransition`] with a
    /// [`run_once`](`run_once_condition`) condition to run the on enter schedule of the
    /// initial state.
    ///
    /// If you would like to control how other systems run based on the current state,
    /// you can emulate this behavior using the [`in_state`] [`Condition`](bevy_ecs::schedule::Condition).
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by adding the [`apply_state_transition`] system manually.
    pub fn add_state<S: States>(&mut self) -> &mut Self {
        self.init_resource::<State<S>>()
            .init_resource::<NextState<S>>()
            .add_systems(
                StateTransition,
                (
                    run_enter_schedule::<S>.run_if(run_once_condition()),
                    apply_state_transition::<S>,
                )
                    .chain(),
            );

        // The OnEnter, OnExit, and OnTransition schedules are lazily initialized
        // (i.e. when the first system is added to them), and World::try_run_schedule is used to fail
        // gracefully if they aren't present.

        self
    }

    /// Adds a system to the given schedule in this app's [`Schedules`].
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
        let mut schedules = self.world.resource_mut::<Schedules>();

        if let Some(schedule) = schedules.get_mut(&schedule) {
            schedule.add_systems(systems);
        } else {
            let mut new_schedule = Schedule::new(schedule);
            new_schedule.add_systems(systems);
            schedules.insert(new_schedule);
        }

        self
    }

    /// Configures a system set in the default schedule, adding the set if it does not exist.
    #[deprecated(since = "0.12.0", note = "Please use `configure_sets` instead.")]
    #[track_caller]
    pub fn configure_set(
        &mut self,
        schedule: impl ScheduleLabel,
        set: impl IntoSystemSetConfigs,
    ) -> &mut Self {
        self.configure_sets(schedule, set)
    }

    /// Configures a collection of system sets in the default schedule, adding any sets that do not exist.
    #[track_caller]
    pub fn configure_sets(
        &mut self,
        schedule: impl ScheduleLabel,
        sets: impl IntoSystemSetConfigs,
    ) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        if let Some(schedule) = schedules.get_mut(&schedule) {
            schedule.configure_sets(sets);
        } else {
            let mut new_schedule = Schedule::new(schedule);
            new_schedule.configure_sets(sets);
            schedules.insert(new_schedule);
        }
        self
    }

    /// Setup the application to manage events of type `T`.
    ///
    /// This is done by adding a [`Resource`] of type [`Events::<T>`],
    /// and inserting an [`event_update_system`] into [`First`].
    ///
    /// See [`Events`] for defining events.
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
    ///
    /// [`event_update_system`]: bevy_ecs::event::event_update_system
    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Event,
    {
        if !self.world.contains_resource::<Events<T>>() {
            self.init_resource::<Events<T>>().add_systems(
                First,
                bevy_ecs::event::event_update_system::<T>
                    .run_if(bevy_ecs::event::event_update_condition::<T>),
            );
        }
        self
    }

    /// Inserts a [`Resource`] to the current [`App`] and overwrites any [`Resource`] previously added of the same type.
    ///
    /// A [`Resource`] in Bevy represents globally unique data. [`Resource`]s must be added to Bevy apps
    /// before using them. This happens with [`insert_resource`](Self::insert_resource).
    ///
    /// See [`init_resource`](Self::init_resource) for [`Resource`]s that implement [`Default`] or [`FromWorld`].
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
        self.world.insert_resource(resource);
        self
    }

    /// Inserts a non-send resource to the app.
    ///
    /// You usually want to use [`insert_resource`](Self::insert_resource),
    /// but there are some special cases when a resource cannot be sent across threads.
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
        self.world.insert_non_send_resource(resource);
        self
    }

    /// Initialize a [`Resource`] with standard starting values by adding it to the [`World`].
    ///
    /// If the [`Resource`] already exists, nothing happens.
    ///
    /// The [`Resource`] must implement the [`FromWorld`] trait.
    /// If the [`Default`] trait is implemented, the [`FromWorld`] trait will use
    /// the [`Default::default`] method to initialize the [`Resource`].
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
        self.world.init_resource::<R>();
        self
    }

    /// Initialize a non-send [`Resource`] with standard starting values by adding it to the [`World`].
    ///
    /// The [`Resource`] must implement the [`FromWorld`] trait.
    /// If the [`Default`] trait is implemented, the [`FromWorld`] trait will use
    /// the [`Default::default`] method to initialize the [`Resource`].
    pub fn init_non_send_resource<R: 'static + FromWorld>(&mut self) -> &mut Self {
        self.world.init_non_send_resource::<R>();
        self
    }

    /// Sets the function that will be called when the app is run.
    ///
    /// The runner function `run_fn` is called only once by [`App::run`]. If the
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
    /// fn my_runner(mut app: App) {
    ///     loop {
    ///         println!("In main loop");
    ///         app.update();
    ///     }
    /// }
    ///
    /// App::new()
    ///     .set_runner(my_runner);
    /// ```
    pub fn set_runner(&mut self, run_fn: impl FnOnce(App) + 'static + Send) -> &mut Self {
        self.runner = Box::new(run_fn);
        self
    }

    /// Boxed variant of [`add_plugins`](App::add_plugins) that can be used from a
    /// [`PluginGroup`](super::PluginGroup)
    pub(crate) fn add_boxed_plugin(
        &mut self,
        plugin: Box<dyn Plugin>,
    ) -> Result<&mut Self, AppError> {
        debug!("added plugin: {}", plugin.name());
        if plugin.is_unique() && !self.plugin_name_added.insert(plugin.name().to_string()) {
            Err(AppError::DuplicatePlugin {
                plugin_name: plugin.name().to_string(),
            })?;
        }

        // Reserve that position in the plugin registry. if a plugin adds plugins, they will be correctly ordered
        let plugin_position_in_registry = self.plugin_registry.len();
        self.plugin_registry.push(Box::new(PlaceholderPlugin));

        self.building_plugin_depth += 1;
        let result = catch_unwind(AssertUnwindSafe(|| plugin.build(self)));
        self.building_plugin_depth -= 1;
        if let Err(payload) = result {
            resume_unwind(payload);
        }
        self.plugin_registry[plugin_position_in_registry] = plugin;
        Ok(self)
    }

    /// Checks if a [`Plugin`] has already been added.
    ///
    /// This can be used by plugins to check if a plugin they depend upon has already been
    /// added.
    pub fn is_plugin_added<T>(&self) -> bool
    where
        T: Plugin,
    {
        self.plugin_registry
            .iter()
            .any(|p| p.downcast_ref::<T>().is_some())
    }

    /// Returns a vector of references to any plugins of type `T` that have been added.
    ///
    /// This can be used to read the settings of any already added plugins.
    /// This vector will be length zero if no plugins of that type have been added.
    /// If multiple copies of the same plugin are added to the [`App`], they will be listed in insertion order in this vector.
    ///
    /// ```rust
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
        self.plugin_registry
            .iter()
            .filter_map(|p| p.downcast_ref())
            .collect()
    }

    /// Adds one or more [`Plugin`]s.
    ///
    /// One of Bevy's core principles is modularity. All Bevy engine features are implemented
    /// as [`Plugin`]s. This includes internal features like the renderer.
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
    /// Panics if one of the plugins was already added to the application.
    ///
    /// [`PluginGroup`]:super::PluginGroup
    #[track_caller]
    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        plugins.add_to_app(self);
        self
    }

    /// Registers the type `T` in the [`TypeRegistry`](bevy_reflect::TypeRegistry) resource,
    /// adding reflect data as specified in the [`Reflect`](bevy_reflect::Reflect) derive:
    /// ```rust,ignore
    /// #[derive(Reflect)]
    /// #[reflect(Component, Serialize, Deserialize)] // will register ReflectComponent, ReflectSerialize, ReflectDeserialize
    /// ```
    ///
    /// See [`bevy_reflect::TypeRegistry::register`].
    #[cfg(feature = "bevy_reflect")]
    pub fn register_type<T: bevy_reflect::GetTypeRegistration>(&mut self) -> &mut Self {
        let registry = self.world.resource_mut::<AppTypeRegistry>();
        registry.write().register::<T>();
        self
    }

    /// Adds the type data `D` to type `T` in the [`TypeRegistry`](bevy_reflect::TypeRegistry) resource.
    ///
    /// Most of the time [`App::register_type`] can be used instead to register a type you derived [`Reflect`](bevy_reflect::Reflect) for.
    /// However, in cases where you want to add a piece of type data that was not included in the list of `#[reflect(...)]` type data in the derive,
    /// or where the type is generic and cannot register e.g. `ReflectSerialize` unconditionally without knowing the specific type parameters,
    /// this method can be used to insert additional type data.
    ///
    /// # Example
    /// ```rust
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
        T: bevy_reflect::Reflect + 'static,
        D: bevy_reflect::TypeData + bevy_reflect::FromType<T>,
    >(
        &mut self,
    ) -> &mut Self {
        let registry = self.world.resource_mut::<AppTypeRegistry>();
        registry.write().register_type_data::<T, D>();
        self
    }

    /// Retrieves a `SubApp` stored inside this [`App`].
    ///
    /// # Panics
    ///
    /// Panics if the `SubApp` doesn't exist.
    pub fn sub_app_mut(&mut self, label: impl AppLabel) -> &mut App {
        match self.get_sub_app_mut(label) {
            Ok(app) => app,
            Err(label) => panic!("Sub-App with label '{:?}' does not exist", label.as_str()),
        }
    }

    /// Retrieves a `SubApp` inside this [`App`] with the given label, if it exists. Otherwise returns
    /// an [`Err`] containing the given label.
    pub fn get_sub_app_mut(&mut self, label: impl AppLabel) -> Result<&mut App, AppLabelId> {
        let label = label.as_label();
        self.sub_apps
            .get_mut(&label)
            .map(|sub_app| &mut sub_app.app)
            .ok_or(label)
    }

    /// Retrieves a `SubApp` stored inside this [`App`].
    ///
    /// # Panics
    ///
    /// Panics if the `SubApp` doesn't exist.
    pub fn sub_app(&self, label: impl AppLabel) -> &App {
        match self.get_sub_app(label) {
            Ok(app) => app,
            Err(label) => panic!("Sub-App with label '{:?}' does not exist", label.as_str()),
        }
    }

    /// Inserts an existing sub app into the app
    pub fn insert_sub_app(&mut self, label: impl AppLabel, sub_app: SubApp) {
        self.sub_apps.insert(label.as_label(), sub_app);
    }

    /// Removes a sub app from the app. Returns [`None`] if the label doesn't exist.
    pub fn remove_sub_app(&mut self, label: impl AppLabel) -> Option<SubApp> {
        self.sub_apps.remove(&label.as_label())
    }

    /// Retrieves a `SubApp` inside this [`App`] with the given label, if it exists. Otherwise returns
    /// an [`Err`] containing the given label.
    pub fn get_sub_app(&self, label: impl AppLabel) -> Result<&App, impl AppLabel> {
        self.sub_apps
            .get(&label.as_label())
            .map(|sub_app| &sub_app.app)
            .ok_or(label)
    }

    /// Adds a new `schedule` to the [`App`] under the provided `label`.
    ///
    /// # Warning
    /// This method will overwrite any existing schedule at that label.
    /// To avoid this behavior, use the `init_schedule` method instead.
    pub fn add_schedule(&mut self, schedule: Schedule) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        schedules.insert(schedule);

        self
    }

    /// Initializes a new empty `schedule` to the [`App`] under the provided `label` if it does not exists.
    ///
    /// See [`App::add_schedule`] to pass in a pre-constructed schedule.
    pub fn init_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        if !schedules.contains(&label) {
            schedules.insert(Schedule::new(label));
        }
        self
    }

    /// Gets read-only access to the [`Schedule`] with the provided `label` if it exists.
    pub fn get_schedule(&self, label: impl ScheduleLabel) -> Option<&Schedule> {
        let schedules = self.world.get_resource::<Schedules>()?;
        schedules.get(&label)
    }

    /// Gets read-write access to a [`Schedule`] with the provided `label` if it exists.
    pub fn get_schedule_mut(&mut self, label: impl ScheduleLabel) -> Option<&mut Schedule> {
        let schedules = self.world.get_resource_mut::<Schedules>()?;
        // We need to call .into_inner here to satisfy the borrow checker:
        // it can reason about reborrows using ordinary references but not the `Mut` smart pointer.
        schedules.into_inner().get_mut(&label)
    }

    /// Applies the function to the [`Schedule`] associated with `label`.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    pub fn edit_schedule(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnOnce(&mut Schedule),
    ) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();

        if schedules.get(&label).is_none() {
            schedules.insert(Schedule::new(label.dyn_clone()));
        }

        let schedule = schedules.get_mut(&label).unwrap();
        // Call the function f, passing in the schedule retrieved
        f(schedule);

        self
    }

    /// Applies the provided [`ScheduleBuildSettings`] to all schedules.
    pub fn configure_schedules(
        &mut self,
        schedule_build_settings: ScheduleBuildSettings,
    ) -> &mut Self {
        self.world
            .resource_mut::<Schedules>()
            .configure_schedules(schedule_build_settings);
        self
    }
}

fn run_once(mut app: App) {
    while !app.ready() {
        #[cfg(not(target_arch = "wasm32"))]
        bevy_tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();

    app.update();
}

/// An event that indicates the [`App`] should exit. This will fully exit the app process at the
/// start of the next tick of the schedule.
///
/// You can also use this event to detect that an exit was requested. In order to receive it, systems
/// subscribing to this event should run after it was emitted and before the schedule of the same
/// frame is over. This is important since [`App::run()`] might never return.
///
/// If you don't require access to other components or resources, consider implementing the [`Drop`]
/// trait on components/resources for code that runs on exit. That saves you from worrying about
/// system schedule ordering, and is idiomatic Rust.
#[derive(Event, Debug, Clone, Default)]
pub struct AppExit;

#[cfg(test)]
mod tests {
    use bevy_ecs::{
        schedule::{OnEnter, States},
        system::Commands,
    };

    use crate::{App, Plugin};

    struct PluginA;
    impl Plugin for PluginA {
        fn build(&self, _app: &mut crate::App) {}
    }
    struct PluginB;
    impl Plugin for PluginB {
        fn build(&self, _app: &mut crate::App) {}
    }
    struct PluginC<T>(T);
    impl<T: Send + Sync + 'static> Plugin for PluginC<T> {
        fn build(&self, _app: &mut crate::App) {}
    }
    struct PluginD;
    impl Plugin for PluginD {
        fn build(&self, _app: &mut crate::App) {}
        fn is_unique(&self) -> bool {
            false
        }
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
            fn build(&self, _: &mut crate::App) {}
        }
        impl Plugin for PluginRun {
            fn build(&self, app: &mut crate::App) {
                app.add_plugins(InnerPlugin).run();
            }
        }
        App::new().add_plugins(PluginRun);
    }

    #[derive(States, PartialEq, Eq, Debug, Default, Hash, Clone)]
    enum AppState {
        #[default]
        MainMenu,
    }
    fn bar(mut commands: Commands) {
        commands.spawn_empty();
    }

    fn foo(mut commands: Commands) {
        commands.spawn_empty();
    }

    #[test]
    fn add_systems_should_create_schedule_if_it_does_not_exist() {
        let mut app = App::new();
        app.add_state::<AppState>()
            .add_systems(OnEnter(AppState::MainMenu), (foo, bar));

        app.world.run_schedule(OnEnter(AppState::MainMenu));
        assert_eq!(app.world.entities().len(), 2);
    }

    #[test]
    fn add_systems_should_create_schedule_if_it_does_not_exist2() {
        let mut app = App::new();
        app.add_systems(OnEnter(AppState::MainMenu), (foo, bar))
            .add_state::<AppState>();

        app.world.run_schedule(OnEnter(AppState::MainMenu));
        assert_eq!(app.world.entities().len(), 2);
    }
}
