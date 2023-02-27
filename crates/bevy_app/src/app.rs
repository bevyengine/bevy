use crate::{
    CoreSchedule, CoreSet, IntoSystemAppConfig, IntoSystemAppConfigs, Plugin, PluginGroup,
    StartupSet, SystemAppConfig,
};
pub use bevy_derive::AppLabel;
use bevy_ecs::{
    prelude::*,
    schedule::{
        apply_state_transition, common_conditions::run_once as run_once_condition,
        run_enter_schedule, BoxedScheduleLabel, IntoSystemConfig, IntoSystemSetConfigs,
        ScheduleLabel,
    },
};
use bevy_utils::{tracing::debug, HashMap, HashSet};
use std::fmt::Debug;

#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
bevy_utils::define_label!(
    /// A strongly-typed class of labels used to identify an [`App`].
    AppLabel,
    /// A strongly-typed identifier for an [`AppLabel`].
    AppLabelId,
);

/// The [`Resource`] that stores the [`App`]'s [`TypeRegistry`](bevy_reflect::TypeRegistry).
#[cfg(feature = "bevy_reflect")]
#[derive(Resource, Clone, bevy_derive::Deref, bevy_derive::DerefMut, Default)]
pub struct AppTypeRegistry(pub bevy_reflect::TypeRegistryArc);

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
///        .add_system(hello_world_system)
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
    pub runner: Box<dyn Fn(App) + Send>, // Send bound is required to make App Send
    /// The schedule that systems are added to by default.
    ///
    /// This is initially set to [`CoreSchedule::Main`].
    pub default_schedule_label: BoxedScheduleLabel,
    /// The schedule that controls the outer loop of schedule execution.
    ///
    /// This is initially set to [`CoreSchedule::Outer`].
    pub outer_schedule_label: BoxedScheduleLabel,
    sub_apps: HashMap<AppLabelId, SubApp>,
    plugin_registry: Vec<Box<dyn Plugin>>,
    plugin_name_added: HashSet<String>,
    /// A private marker to prevent incorrect calls to `App::run()` from `Plugin::build()`
    is_building_plugin: bool,
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
/// # use bevy_app::{App, AppLabel, SubApp, CoreSchedule};
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
/// sub_app.add_simple_outer_schedule();
/// sub_app.insert_resource(Val(100));
///
/// // initialize main schedule
/// sub_app.init_schedule(CoreSchedule::Main);
/// sub_app.add_system(|counter: Res<Val>| {
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

    /// A function that allows access to both the [`SubApp`] [`World`] and the main [`App`]. This is
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

    /// Runs the `SubApp`'s default schedule.
    pub fn run(&mut self) {
        self.app
            .world
            .run_schedule_ref(&*self.app.outer_schedule_label);
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

        app.add_default_schedules();

        app.add_event::<AppExit>();

        #[cfg(feature = "bevy_ci_testing")]
        {
            crate::ci_testing::setup_app(&mut app);
        }

        app
    }
}

impl App {
    /// Creates a new [`App`] with some default structure to enable core engine features.
    /// This is the preferred constructor for most use cases.
    ///
    /// This calls [`App::add_default_schedules`].
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
            default_schedule_label: Box::new(CoreSchedule::Main),
            outer_schedule_label: Box::new(CoreSchedule::Outer),
            is_building_plugin: false,
        }
    }

    /// Advances the execution of the [`Schedule`] by one cycle.
    ///
    /// This method also updates sub apps.
    /// See [`insert_sub_app`](Self::insert_sub_app) for more details.
    ///
    /// The schedule run by this method is determined by the [`outer_schedule_label`](App) field.
    /// In normal usage, this is [`CoreSchedule::Outer`], which will run [`CoreSchedule::Startup`]
    /// the first time the app is run, then [`CoreSchedule::Main`] on every call of this method.
    ///
    /// # Panics
    ///
    /// The active schedule of the app must be set before this method is called.
    pub fn update(&mut self) {
        {
            #[cfg(feature = "trace")]
            let _bevy_frame_update_span = info_span!("main app").entered();
            self.world.run_schedule_ref(&*self.outer_schedule_label);
        }
        for (_label, sub_app) in self.sub_apps.iter_mut() {
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
    /// window is closed and that event loop terminates – behaviour of processes that
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
        if app.is_building_plugin {
            panic!("App::run() was called from within Plugin::Build(), which is not allowed.");
        }

        Self::setup(&mut app);

        let runner = std::mem::replace(&mut app.runner, Box::new(run_once));
        (runner)(app);
    }

    /// Run [`Plugin::setup`] for each plugin. This is usually called by [`App::run`], but can
    /// be useful for situations where you want to use [`App::update`].
    pub fn setup(&mut self) {
        // temporarily remove the plugin registry to run each plugin's setup function on app.
        let plugin_registry = std::mem::take(&mut self.plugin_registry);
        for plugin in &plugin_registry {
            plugin.setup(self);
        }
        self.plugin_registry = plugin_registry;
    }

    /// Adds [`State<S>`] and [`NextState<S>`] resources, [`OnEnter`] and [`OnExit`] schedules
    /// for each state variant, an instance of [`apply_state_transition::<S>`] in
    /// [`CoreSet::StateTransitions`] so that transitions happen before [`CoreSet::Update`] and
    /// a instance of [`run_enter_schedule::<S>`] in [`CoreSet::StateTransitions`] with a
    /// [`run_once`](`run_once_condition`) condition to run the on enter schedule of the
    /// initial state.
    ///
    /// This also adds an [`OnUpdate`] system set for each state variant,
    /// which runs during [`CoreSet::Update`] after the transitions are applied.
    /// These system sets only run if the [`State<S>`] resource matches the respective state variant.
    ///
    /// If you would like to control how other systems run based on the current state,
    /// you can emulate this behavior using the [`in_state`] [`Condition`](bevy_ecs::schedule::Condition).
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by adding the [`apply_state_transition`] system manually.
    pub fn add_state<S: States>(&mut self) -> &mut Self {
        self.init_resource::<State<S>>();
        self.init_resource::<NextState<S>>();

        let mut schedules = self.world.resource_mut::<Schedules>();

        let Some(default_schedule) = schedules.get_mut(&*self.default_schedule_label) else {
            let schedule_label = &self.default_schedule_label;
            panic!("Default schedule {schedule_label:?} does not exist.")
        };

        default_schedule.add_systems(
            (
                run_enter_schedule::<S>.run_if(run_once_condition()),
                apply_state_transition::<S>,
            )
                .chain()
                .in_base_set(CoreSet::StateTransitions),
        );

        for variant in S::variants() {
            default_schedule.configure_set(
                OnUpdate(variant.clone())
                    .in_base_set(CoreSet::Update)
                    .run_if(in_state(variant)),
            );
        }

        // These are different for loops to avoid conflicting access to self
        for variant in S::variants() {
            self.add_schedule(OnEnter(variant.clone()), Schedule::new());
            self.add_schedule(OnExit(variant), Schedule::new());
        }

        self
    }

    /// Adds a system to the default system set and schedule of the app's [`Schedules`].
    ///
    /// Refer to the [system module documentation](bevy_ecs::system) to see how a system
    /// can be defined.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # fn my_system() {}
    /// # let mut app = App::new();
    /// #
    /// app.add_system(my_system);
    /// ```
    pub fn add_system<M>(&mut self, system: impl IntoSystemAppConfig<M>) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();

        let SystemAppConfig { system, schedule } = system.into_app_config();

        if let Some(schedule_label) = schedule {
            if let Some(schedule) = schedules.get_mut(&*schedule_label) {
                schedule.add_system(system);
            } else {
                panic!("Schedule {schedule_label:?} does not exist.")
            }
        } else if let Some(default_schedule) = schedules.get_mut(&*self.default_schedule_label) {
            default_schedule.add_system(system);
        } else {
            let schedule_label = &self.default_schedule_label;
            panic!("Default schedule {schedule_label:?} does not exist.")
        }

        self
    }

    /// Adds a system to the default system set and schedule of the app's [`Schedules`].
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
    /// #
    /// app.add_systems((system_a, system_b, system_c));
    /// ```
    pub fn add_systems<M>(&mut self, systems: impl IntoSystemAppConfigs<M>) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();

        match systems.into_app_configs().0 {
            crate::InnerConfigs::Blanket { systems, schedule } => {
                let schedule = if let Some(label) = schedule {
                    schedules
                        .get_mut(&*label)
                        .unwrap_or_else(|| panic!("Schedule '{label:?}' does not exist."))
                } else {
                    let label = &*self.default_schedule_label;
                    schedules
                        .get_mut(label)
                        .unwrap_or_else(|| panic!("Default schedule '{label:?}' does not exist."))
                };
                schedule.add_systems(systems);
            }
            crate::InnerConfigs::Granular(systems) => {
                for system in systems {
                    self.add_system(system);
                }
            }
        }

        self
    }

    /// Adds a system to [`CoreSchedule::Startup`].
    ///
    /// These systems will run exactly once, at the start of the [`App`]'s lifecycle.
    /// To add a system that runs every frame, see [`add_system`](Self::add_system).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// fn my_startup_system(_commands: Commands) {
    ///     println!("My startup system");
    /// }
    ///
    /// App::new()
    ///     .add_startup_system(my_startup_system);
    /// ```
    pub fn add_startup_system<M>(&mut self, system: impl IntoSystemConfig<M>) -> &mut Self {
        self.add_system(system.in_schedule(CoreSchedule::Startup))
    }

    /// Adds a collection of systems to [`CoreSchedule::Startup`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut app = App::new();
    /// # fn startup_system_a() {}
    /// # fn startup_system_b() {}
    /// # fn startup_system_c() {}
    /// #
    /// app.add_startup_systems((
    ///     startup_system_a,
    ///     startup_system_b,
    ///     startup_system_c,
    /// ));
    /// ```
    pub fn add_startup_systems<M>(&mut self, systems: impl IntoSystemConfigs<M>) -> &mut Self {
        self.add_systems(systems.into_configs().in_schedule(CoreSchedule::Startup))
    }

    /// Configures a system set in the default schedule, adding the set if it does not exist.
    pub fn configure_set(&mut self, set: impl IntoSystemSetConfig) -> &mut Self {
        self.world
            .resource_mut::<Schedules>()
            .get_mut(&*self.default_schedule_label)
            .unwrap()
            .configure_set(set);
        self
    }

    /// Configures a collection of system sets in the default schedule, adding any sets that do not exist.
    pub fn configure_sets(&mut self, sets: impl IntoSystemSetConfigs) -> &mut Self {
        self.world
            .resource_mut::<Schedules>()
            .get_mut(&*self.default_schedule_label)
            .unwrap()
            .configure_sets(sets);
        self
    }

    /// Adds standardized schedules and labels to an [`App`].
    ///
    /// Adding these schedules is necessary to make almost all core engine features work.
    ///  This is typically done implicitly by calling `App::default`, which is in turn called by
    /// [`App::new`].
    ///
    /// The schedules added are defined in the [`CoreSchedule`] enum,
    /// and have a starting configuration defined by:
    ///
    /// - [`CoreSchedule::Outer`]: uses [`CoreSchedule::outer_schedule`]
    /// - [`CoreSchedule::Startup`]: uses [`StartupSet::base_schedule`]
    /// - [`CoreSchedule::Main`]: uses [`CoreSet::base_schedule`]
    /// - [`CoreSchedule::FixedUpdate`]: no starting configuration
    ///
    /// # Examples
    ///
    /// ```
    /// use bevy_app::App;
    /// use bevy_ecs::schedule::Schedules;
    ///
    /// let app = App::empty()
    ///     .init_resource::<Schedules>()
    ///     .add_default_schedules()
    ///     .update();
    /// ```
    pub fn add_default_schedules(&mut self) -> &mut Self {
        self.add_schedule(CoreSchedule::Outer, CoreSchedule::outer_schedule());
        self.add_schedule(CoreSchedule::Startup, StartupSet::base_schedule());
        self.add_schedule(CoreSchedule::Main, CoreSet::base_schedule());
        self.init_schedule(CoreSchedule::FixedUpdate);

        self
    }

    /// adds a single threaded outer schedule to the [`App`] that just runs the main schedule
    pub fn add_simple_outer_schedule(&mut self) -> &mut Self {
        fn run_main_schedule(world: &mut World) {
            world.run_schedule(CoreSchedule::Main);
        }

        self.edit_schedule(CoreSchedule::Outer, |schedule| {
            schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::SingleThreaded);
            schedule.add_system(run_main_schedule);
        });

        self
    }

    /// Setup the application to manage events of type `T`.
    ///
    /// This is done by adding a [`Resource`] of type [`Events::<T>`],
    /// and inserting an [`update_system`](Events::update_system) into [`CoreSet::First`].
    ///
    /// See [`Events`] for defining events.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # struct MyEvent;
    /// # let mut app = App::new();
    /// #
    /// app.add_event::<MyEvent>();
    /// ```
    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Event,
    {
        if !self.world.contains_resource::<Events<T>>() {
            self.init_resource::<Events<T>>()
                .add_system(Events::<T>::update_system.in_base_set(CoreSet::First));
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
    pub fn set_runner(&mut self, run_fn: impl Fn(App) + 'static + Send) -> &mut Self {
        self.runner = Box::new(run_fn);
        self
    }

    /// Adds a single [`Plugin`].
    ///
    /// One of Bevy's core principles is modularity. All Bevy engine features are implemented
    /// as [`Plugin`]s. This includes internal features like the renderer.
    ///
    /// Bevy also provides a few sets of default [`Plugin`]s. See [`add_plugins`](Self::add_plugins).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// #
    /// # // Dummies created to avoid using `bevy_log`,
    /// # // which pulls in too many dependencies and breaks rust-analyzer
    /// # pub mod bevy_log {
    /// #     use bevy_app::prelude::*;
    /// #     #[derive(Default)]
    /// #     pub struct LogPlugin;
    /// #     impl Plugin for LogPlugin{
    /// #        fn build(&self, app: &mut App) {}
    /// #     }
    /// # }
    /// App::new().add_plugin(bevy_log::LogPlugin::default());
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the plugin was already added to the application.
    pub fn add_plugin<T>(&mut self, plugin: T) -> &mut Self
    where
        T: Plugin,
    {
        match self.add_boxed_plugin(Box::new(plugin)) {
            Ok(app) => app,
            Err(AppError::DuplicatePlugin { plugin_name }) => panic!(
                "Error adding plugin {plugin_name}: : plugin was already added in application"
            ),
        }
    }

    /// Boxed variant of `add_plugin`, can be used from a [`PluginGroup`]
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
        self.is_building_plugin = true;
        plugin.build(self);
        self.is_building_plugin = false;
        self.plugin_registry.push(plugin);
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
    /// # app.add_plugin(ImagePlugin::default());
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

    /// Adds a group of [`Plugin`]s.
    ///
    /// [`Plugin`]s can be grouped into a set by using a [`PluginGroup`].
    ///
    /// There are built-in [`PluginGroup`]s that provide core engine functionality.
    /// The [`PluginGroup`]s available by default are `DefaultPlugins` and `MinimalPlugins`.
    ///
    /// To customize the plugins in the group (reorder, disable a plugin, add a new plugin
    /// before / after another plugin), call [`build()`](PluginGroup::build) on the group,
    /// which will convert it to a [`PluginGroupBuilder`](crate::PluginGroupBuilder).
    ///
    /// ## Examples
    /// ```
    /// # use bevy_app::{prelude::*, PluginGroupBuilder, NoopPluginGroup as MinimalPlugins};
    /// #
    /// App::new()
    ///     .add_plugins(MinimalPlugins);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if one of the plugin in the group was already added to the application.
    pub fn add_plugins<T: PluginGroup>(&mut self, group: T) -> &mut Self {
        let builder = group.build();
        builder.finish(self);
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
    pub fn add_schedule(&mut self, label: impl ScheduleLabel, schedule: Schedule) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        schedules.insert(label, schedule);

        self
    }

    /// Initializes a new empty `schedule` to the [`App`] under the provided `label` if it does not exists.
    ///
    /// See [`App::add_schedule`] to pass in a pre-constructed schedule.
    pub fn init_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        if !schedules.contains(&label) {
            schedules.insert(label, Schedule::new());
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
        mut f: impl FnMut(&mut Schedule),
    ) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();

        if schedules.get(&label).is_none() {
            schedules.insert(label.dyn_clone(), Schedule::new());
        }

        let schedule = schedules.get_mut(&label).unwrap();
        // Call the function f, passing in the schedule retrieved
        f(schedule);

        self
    }
}

fn run_once(mut app: App) {
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
#[derive(Debug, Clone, Default)]
pub struct AppExit;

#[cfg(test)]
mod tests {
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
        App::new().add_plugin(PluginA).add_plugin(PluginB);
    }

    #[test]
    #[should_panic]
    fn cant_add_twice_the_same_plugin() {
        App::new().add_plugin(PluginA).add_plugin(PluginA);
    }

    #[test]
    fn can_add_twice_the_same_plugin_with_different_type_param() {
        App::new().add_plugin(PluginC(0)).add_plugin(PluginC(true));
    }

    #[test]
    fn can_add_twice_the_same_plugin_not_unique() {
        App::new().add_plugin(PluginD).add_plugin(PluginD);
    }

    #[test]
    #[should_panic]
    fn cant_call_app_run_from_plugin_build() {
        struct PluginRun;
        impl Plugin for PluginRun {
            fn build(&self, app: &mut crate::App) {
                app.run();
            }
        }
        App::new().add_plugin(PluginRun);
    }
}
