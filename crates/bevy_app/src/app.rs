use crate::{CoreSchedule, CoreSet, Plugin, PluginGroup, StartupSchedule, StartupSet};
pub use bevy_derive::AppLabel;
use bevy_ecs::{
    event::{Event, Events},
    prelude::FromWorld,
    schedule::{
        apply_system_buffers, IntoSystemConfig, IntoSystemConfigs, IntoSystemDescriptor, Schedule,
        ScheduleLabel, Schedules, ShouldRun, State, StateData, SystemSet,
    },
    system::Resource,
    world::World,
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
    /// If additional separate [`World`]-[`Schedule`] pairs are needed, you can use [`sub_app`](App::add_sub_app)s.
    pub world: World,
    /// The [runner function](Self::set_runner) is primarily responsible for managing
    /// the application's event loop and advancing the [`Schedule`].
    /// Typically, it is not configured manually, but set by one of Bevy's built-in plugins.
    /// See `bevy::winit::WinitPlugin` and [`ScheduleRunnerPlugin`](crate::schedule_runner::ScheduleRunnerPlugin).
    pub runner: Box<dyn Fn(App)>,
    /// A collection of [`Schedule`] objects which are used to run systems.
    pub schedules: Schedules,
    /// The [`Schedule`] that systems will be added to by default.
    default_schedule: Option<Box<dyn ScheduleLabel>>,
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

/// Each `SubApp` has its own [`Schedule`] and [`World`], enabling a separation of concerns.
struct SubApp {
    app: App,
    extract: Box<dyn Fn(&mut World, &mut App)>,
}

impl SubApp {
    /// Runs the `SubApp`'s schedule.
    pub fn run(&mut self) {
        self.app.schedule.run(&mut self.app.world);
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
        app.add_default_sets();

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
    /// This calls [`App::add_default_schedules`] and [App::add_defaults_sets`].
    pub fn new() -> App {
        App::default()
    }

    /// Creates a new empty [`App`] with minimal default configuration.
    ///
    /// This constructor should be used if you wish to provide a custom schedule, exit handling, cleanup, etc.
    pub fn empty() -> App {
        Self {
            world: Default::default(),
            default_schedule: None,
            schedules: Default::default(),
            runner: Box::new(run_once),
            sub_apps: HashMap::default(),
            plugin_registry: Vec::default(),
            plugin_name_added: Default::default(),
            is_building_plugin: false,
        }
    }

    /// Advances the execution of the [`Schedule`] by one cycle.
    ///
    /// This method also updates sub apps.
    ///
    /// See [`add_sub_app`](Self::add_sub_app) and [`run_once`](Schedule::run_once) for more details.
    pub fn update(&mut self) {
        #[cfg(feature = "trace")]
        let _bevy_frame_update_span = info_span!("frame").entered();
        self.schedule.run(&mut self.world);

        for sub_app in self.sub_apps.values_mut() {
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

        // temporarily remove the plugin registry to run each plugin's setup function on app.
        let mut plugin_registry = std::mem::take(&mut app.plugin_registry);
        for plugin in &plugin_registry {
            plugin.setup(&mut app);
        }
        std::mem::swap(&mut app.plugin_registry, &mut plugin_registry);

        let runner = std::mem::replace(&mut app.runner, Box::new(run_once));
        (runner)(app);
    }

    /// Sets the [`Schedule`] that will be modified by default when you call `App::add_system`
    /// and similar methods.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    pub fn set_default_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        self.default_schedule = Some(Box::new(label));
        if self.schedules.get(&label).is_none() {
            self.schedules.insert(label, Schedule::new());
        }

        self
    }

    /// Gets the label of the [`Schedule`] that will be modified by default when you call `App::add_system`
    /// and similar methods.
    pub fn default_schedule(&mut self, label: impl ScheduleLabel) -> &Box<dyn ScheduleLabel> {
        &self.default_schedule
    }

    /// Applies the function to the [`Schedule`] associated with `label`.
    ///
    /// **Note:** This will create the schedule if it does not already exist.
    fn edit_schedule(
        &mut self,
        label: impl ScheduleLabel,
        f: impl FnMut(&mut Schedule),
    ) -> &mut Self;
    /// Adds [`State<S>`] and [`NextState<S>`] resources, [`OnEnter`] and [`OnExit`] schedules
    /// for each state variant, and an instance of [`apply_state_transition::<S>`] in
    /// \<insert-`bevy_core`-set-name\> so that transitions happen before `Update`.
    fn add_state<S: States>(&mut self) -> &mut Self;

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
    pub fn add_system<P>(&mut self, system: impl IntoSystemConfig<P>) -> &mut Self {
        if let Some(default_schedule_label) = self.default_schedule {
            if let Some(default_schedule) = self.schedules.get_mut(&default_schedule_label) {
                default_schedule.add_system(system);
            } else {
                panic!("Default schedule does not exist.")
            }
        } else {
            panic!("No default schedule set for the `App`.")
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
    /// app.add_system_set(
    ///     SystemSet::new()
    ///         .with_system(system_a)
    ///         .with_system(system_b)
    ///         .with_system(system_c),
    /// );
    /// ```
    pub fn add_systems<P>(&mut self, systems: impl IntoSystemConfigs<P>) -> &mut Self {
        if let Some(default_schedule_label) = self.default_schedule {
            if let Some(default_schedule) = self.schedules.get_mut(&default_schedule_label) {
                default_schedule.add_systems(systems);
            } else {
                panic!("Default schedule does not exist.")
            }
        } else {
            panic!("No default schedule set for the `App`.")
        }

        self
    }

    /// Adds a system to the provided [`Schedule`].
    pub fn add_system_to_schedule<P>(
        &mut self,
        system: impl IntoSystemConfig<P>,
        schedule_label: impl ScheduleLabel,
    ) -> &mut Self {
        if let Some(schedule) = self.schedules.get_mut(&schedule_label) {
            schedule.add_system(system);
        } else {
            panic!("Provided schedule {schedule_label:?} does not exist.")
        }

        self
    }

    /// Adds a collection of system to the provided [`Schedule`].
    pub fn add_systems_to_schedule<P>(
        &mut self,
        systems: impl IntoSystemConfigs<P>,
        schedule_label: impl ScheduleLabel,
    ) -> &mut Self {
        if let Some(schedule) = self.schedules.get_mut(&schedule_label) {
            schedule.add_systems(systems);
        } else {
            panic!("Provided schedule {schedule_label:?} does not exist.")
        }

        self
    }

    /// Adds a system to [`CoreSchedule::Startup`].
    ///
    /// * For adding a system that runs every frame, see [`add_system`](Self::add_system).
    /// * For adding a system to a specific stage, see [`add_system_to_stage`](Self::add_system_to_stage).
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
    pub fn add_startup_system<P>(&mut self, system: impl IntoSystemConfig<P>) -> &mut Self {
        self.add_system_to_schedule(system, CoreSchedule::Startup)
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
    /// app.add_startup_system_set(
    ///     SystemSet::new()
    ///         .with_system(startup_system_a)
    ///         .with_system(startup_system_b)
    ///         .with_system(startup_system_c),
    /// );
    /// ```
    pub fn add_startup_systems<P>(&mut self, systems: impl IntoSystemConfigs<P>) -> &mut Self {
        self.add_systems_to_schedule(systems, CoreSchedule::Startup)
    }

    /// Adds standardized schedules and labels to an [`App`].
    ///
    /// Adding these schedules is necessary to make some core engine features work.
    ///  This is however done by default by calling `App::default`, which is in turn called by
    /// [`App::new`].
    ///
    /// The schedules are defined in the [`CoreSchedule`] enum.
    ///
    /// The [default schedule](App::set_default_schedule) becomes [`CoreSchedule::Main`].
    ///
    /// You can also add standardized system sets to these schedules using [`App::add_default_sets`],
    /// which must be called *after* this method as it relies on these schedules existing.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// #
    /// let app = App::empty().add_default_schedules();
    /// ```
    pub fn add_default_schedules(&mut self) -> &mut Self {
        self.schedules
            .insert(CoreSchedule::Startup, Schedule::new());
        self.schedules.insert(CoreSchedule::Main, Schedule::new());

        self.set_default_schedule(CoreSchedule::Main);

        self
    }

    /// Adds broad system sets to the default [`Schedule`], giving it a standardized linear structure.
    ///
    /// See [`CoreSet`] and [`StartupSet`] for documentation on the system sets added.
    ///
    /// The [default sets](bevy_ecs::schedule::Schedule::set_default_set) becomes [`CoreSet::Update`] and [`StartupSet::Startup`] respectively.
    /// [`Command`](bevy_ecs::prelude::Commands) flush points, set with [`apply_system_buffers`] are added between each of these sets and at the end of each schedule.
    ///
    /// # Panics
    ///
    /// The [`CoreSchedule`] schedules must have been added to the [`App`] before this method is called.
    /// You can do so using [`App::add_default_schedules`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// #
    /// let app = App::empty().add_default_sets();
    /// ```
    pub fn add_default_sets(&mut self) -> &mut Self {
        // Adding sets
        let startup_schedule = self.schedules.get(CoreSchedule::Startup).unwrap();
        startup_schedule.add_set(StartupSet::PreStartup);
        startup_schedule.add_set(StartupSet::Startup);
        startup_schedule.add_set(StartupSet::PostStartup);

        // Ordering
        startup_schedule.configure_set(StartupSet::PreStartup.before(StartupSet::Startup));
        startup_schedule.configure_set(StartupSet::PostStartup.after(StartupSet::Startup));

        // Flush points
        startup_schedule.add_system(
            apply_system_buffers
                .after(StartupSet::PreStartup)
                .before(StartupSet::Startup),
        );
        startup_schedule.add_system(
            apply_system_buffers
                .after(StartupSet::Startup)
                .before(StartupSet::PostStartup),
        );

        startup_schedule.add_system(apply_system_buffers.after(StartupSet::PostStartup));

        // Default set
        startup_schedule.set_default_set(StartupSet::Startup);

        // Adding sets
        let main_schedule = self.schedules.get(CoreSchedule::Startup).unwrap();
        main_schedule.add_set(CoreSet::First);
        main_schedule.add_set(CoreSet::PreUpdate);
        main_schedule.add_set(CoreSet::Update);
        main_schedule.add_set(CoreSet::PostUpdate);
        main_schedule.add_set(CoreSet::Last);

        // Ordering
        main_schedule.configure_set(CoreSet::First.before(CoreSet::PreUpdate));
        main_schedule.configure_set(CoreSet::PreUpdate.before(CoreSet::Update));
        main_schedule.configure_set(CoreSet::PostUpdate.after(CoreSet::Update));
        main_schedule.configure_set(CoreSet::Last.after(CoreSet::PostUpdate));

        // Flush points
        startup_schedule.add_system(
            apply_system_buffers
                .after(CoreSet::First)
                .before(CoreSet::PreUpdate),
        );
        startup_schedule.add_system(
            apply_system_buffers
                .after(CoreSet::PreUpdate)
                .before(CoreSet::Update),
        );

        startup_schedule.add_system(
            apply_system_buffers
                .after(CoreSet::Update)
                .before(CoreSet::PostUpdate),
        );

        startup_schedule.add_system(
            apply_system_buffers
                .after(CoreSet::PostUpdate)
                .before(CoreSet::Last),
        );

        startup_schedule.add_system(apply_system_buffers.after(CoreSet::Last));

        // Default set
        startup_schedule.set_default_set(CoreSet::Update);
    }

    /// Setup the application to manage events of type `T`.
    ///
    /// This is done by adding a [`Resource`] of type [`Events::<T>`],
    /// and inserting an [`update_system`](Events::update_system) into [`CoreStage::First`].
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
                .add_system_to_stage(CoreSet::First, Events::<T>::update_system);
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
    pub fn set_runner(&mut self, run_fn: impl Fn(App) + 'static) -> &mut Self {
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
        {
            let registry = self.world.resource_mut::<AppTypeRegistry>();
            registry.write().register::<T>();
        }
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
        {
            let registry = self.world.resource_mut::<AppTypeRegistry>();
            registry.write().register_type_data::<T, D>();
        }
        self
    }

    /// Adds an [`App`] as a child of the current one.
    ///
    /// The provided function `sub_app_runner` is called by the [`update`](Self::update) method. The [`World`]
    /// parameter represents the main app world, while the [`App`] parameter is just a mutable
    /// reference to the `SubApp` itself.
    pub fn add_sub_app(
        &mut self,
        label: impl AppLabel,
        app: App,
        extract: impl Fn(&mut World, &mut App) + 'static,
    ) -> &mut Self {
        self.sub_apps.insert(
            label.as_label(),
            SubApp {
                app,
                extract: Box::new(extract),
            },
        );
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

    /// Retrieves a `SubApp` inside this [`App`] with the given label, if it exists. Otherwise returns
    /// an [`Err`] containing the given label.
    pub fn get_sub_app(&self, label: impl AppLabel) -> Result<&App, impl AppLabel> {
        self.sub_apps
            .get(&label.as_label())
            .map(|sub_app| &sub_app.app)
            .ok_or(label)
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
/// frame is over.
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
