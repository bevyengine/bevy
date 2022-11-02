use crate::{CoreStage, Plugin, PluginGroup, StartupSchedule, StartupStage};
pub use bevy_derive::AppLabel;
use bevy_ecs::{
    event::{Event, Events},
    prelude::FromWorld,
    schedule::{
        IntoSystemDescriptor, Schedule, ShouldRun, Stage, StageLabel, State, StateData, SystemSet,
        SystemStage,
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
    /// A container of [`Stage`]s set to be run in a linear order.
    pub schedule: Schedule,
    sub_apps: HashMap<AppLabelId, SubApp>,
    plugin_registry: Vec<Box<dyn Plugin>>,
    plugin_name_added: HashSet<String>,
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
    runner: Box<dyn Fn(&mut World, &mut App)>,
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

        app.add_default_stages()
            .add_event::<AppExit>()
            .add_system_to_stage(CoreStage::Last, World::clear_trackers);

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
    pub fn new() -> App {
        App::default()
    }

    /// Creates a new empty [`App`] with minimal default configuration.
    ///
    /// This constructor should be used if you wish to provide a custom schedule, exit handling, cleanup, etc.
    pub fn empty() -> App {
        Self {
            world: Default::default(),
            schedule: Default::default(),
            runner: Box::new(run_once),
            sub_apps: HashMap::default(),
            plugin_registry: Vec::default(),
            plugin_name_added: Default::default(),
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
            (sub_app.runner)(&mut self.world, &mut sub_app.app);
        }
    }

    /// Starts the application by calling the app's [runner function](Self::set_runner).
    ///
    /// Finalizes the [`App`] configuration. For general usage, see the example on the item
    /// level documentation.
    pub fn run(&mut self) {
        #[cfg(feature = "trace")]
        let _bevy_app_run_span = info_span!("bevy_app").entered();

        let mut app = std::mem::replace(self, App::empty());
        let runner = std::mem::replace(&mut app.runner, Box::new(run_once));
        (runner)(app);
    }

    /// Adds a [`Stage`] with the given `label` to the last position of the app's
    /// [`Schedule`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut app = App::new();
    /// #
    /// #[derive(StageLabel)]
    /// struct MyStage;
    /// app.add_stage(MyStage, SystemStage::parallel());
    /// ```
    pub fn add_stage<S: Stage>(&mut self, label: impl StageLabel, stage: S) -> &mut Self {
        self.schedule.add_stage(label, stage);
        self
    }

    /// Adds a [`Stage`] with the given `label` to the app's [`Schedule`], located
    /// immediately after the stage labeled by `target`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut app = App::new();
    /// #
    /// #[derive(StageLabel)]
    /// struct MyStage;
    /// app.add_stage_after(CoreStage::Update, MyStage, SystemStage::parallel());
    /// ```
    pub fn add_stage_after<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        self.schedule.add_stage_after(target, label, stage);
        self
    }

    /// Adds a [`Stage`] with the given `label` to the app's [`Schedule`], located
    /// immediately before the stage labeled by `target`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut app = App::new();
    /// #
    /// #[derive(StageLabel)]
    /// struct MyStage;
    /// app.add_stage_before(CoreStage::Update, MyStage, SystemStage::parallel());
    /// ```
    pub fn add_stage_before<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        self.schedule.add_stage_before(target, label, stage);
        self
    }

    /// Adds a [`Stage`] with the given `label` to the last position of the
    /// [startup schedule](Self::add_default_stages).
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut app = App::new();
    /// #
    /// #[derive(StageLabel)]
    /// struct MyStartupStage;
    /// app.add_startup_stage(MyStartupStage, SystemStage::parallel());
    /// ```
    pub fn add_startup_stage<S: Stage>(&mut self, label: impl StageLabel, stage: S) -> &mut Self {
        self.schedule
            .stage(StartupSchedule, |schedule: &mut Schedule| {
                schedule.add_stage(label, stage)
            });
        self
    }

    /// Adds a [startup stage](Self::add_default_stages) with the given `label`, immediately
    /// after the stage labeled by `target`.
    ///
    /// The `target` label must refer to a stage inside the startup schedule.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut app = App::new();
    /// #
    /// #[derive(StageLabel)]
    /// struct MyStartupStage;
    /// app.add_startup_stage_after(
    ///     StartupStage::Startup,
    ///     MyStartupStage,
    ///     SystemStage::parallel()
    /// );
    /// ```
    pub fn add_startup_stage_after<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        self.schedule
            .stage(StartupSchedule, |schedule: &mut Schedule| {
                schedule.add_stage_after(target, label, stage)
            });
        self
    }

    /// Adds a [startup stage](Self::add_default_stages) with the given `label`, immediately
    /// before the stage labeled by `target`.
    ///
    /// The `target` label must refer to a stage inside the startup schedule.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # let mut app = App::new();
    /// #
    /// #[derive(StageLabel)]
    /// struct MyStartupStage;
    /// app.add_startup_stage_before(
    ///     StartupStage::Startup,
    ///     MyStartupStage,
    ///     SystemStage::parallel()
    /// );
    /// ```
    pub fn add_startup_stage_before<S: Stage>(
        &mut self,
        target: impl StageLabel,
        label: impl StageLabel,
        stage: S,
    ) -> &mut Self {
        self.schedule
            .stage(StartupSchedule, |schedule: &mut Schedule| {
                schedule.add_stage_before(target, label, stage)
            });
        self
    }

    /// Fetches the [`Stage`] of type `T` marked with `label` from the [`Schedule`], then
    /// executes the provided `func` passing the fetched stage to it as an argument.
    ///
    /// The `func` argument should be a function or a closure that accepts a mutable reference
    /// to a struct implementing `Stage` and returns the same type. That means that it should
    /// also assume that the stage has already been fetched successfully.
    ///
    /// See [`stage`](Schedule::stage) for more details.
    ///
    /// # Examples
    ///
    /// Here the closure is used to add a system to the update stage:
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut app = App::new();
    /// # fn my_system() {}
    /// #
    /// app.stage(CoreStage::Update, |stage: &mut SystemStage| {
    ///     stage.add_system(my_system)
    /// });
    /// ```
    pub fn stage<T: Stage, F: FnOnce(&mut T) -> &mut T>(
        &mut self,
        label: impl StageLabel,
        func: F,
    ) -> &mut Self {
        self.schedule.stage(label, func);
        self
    }

    /// Adds a system to the [update stage](Self::add_default_stages) of the app's [`Schedule`].
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
    pub fn add_system<Params>(&mut self, system: impl IntoSystemDescriptor<Params>) -> &mut Self {
        self.add_system_to_stage(CoreStage::Update, system)
    }

    /// Adds a [`SystemSet`] to the [update stage](Self::add_default_stages).
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
    pub fn add_system_set(&mut self, system_set: SystemSet) -> &mut Self {
        self.add_system_set_to_stage(CoreStage::Update, system_set)
    }

    /// Adds a system to the [`Stage`] identified by `stage_label`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut app = App::new();
    /// # fn my_system() {}
    /// #
    /// app.add_system_to_stage(CoreStage::PostUpdate, my_system);
    /// ```
    pub fn add_system_to_stage<Params>(
        &mut self,
        stage_label: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        use std::any::TypeId;
        assert!(
            stage_label.type_id() != TypeId::of::<StartupStage>(),
            "use `add_startup_system_to_stage` instead of `add_system_to_stage` to add a system to a StartupStage"
        );
        self.schedule.add_system_to_stage(stage_label, system);
        self
    }

    /// Adds a [`SystemSet`] to the [`Stage`] identified by `stage_label`.
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
    /// app.add_system_set_to_stage(
    ///     CoreStage::PostUpdate,
    ///     SystemSet::new()
    ///         .with_system(system_a)
    ///         .with_system(system_b)
    ///         .with_system(system_c),
    /// );
    /// ```
    pub fn add_system_set_to_stage(
        &mut self,
        stage_label: impl StageLabel,
        system_set: SystemSet,
    ) -> &mut Self {
        use std::any::TypeId;
        assert!(
            stage_label.type_id() != TypeId::of::<StartupStage>(),
            "use `add_startup_system_set_to_stage` instead of `add_system_set_to_stage` to add system sets to a StartupStage"
        );
        self.schedule
            .add_system_set_to_stage(stage_label, system_set);
        self
    }

    /// Adds a system to the [startup stage](Self::add_default_stages) of the app's [`Schedule`].
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
    pub fn add_startup_system<Params>(
        &mut self,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.add_startup_system_to_stage(StartupStage::Startup, system)
    }

    /// Adds a [`SystemSet`] to the [startup stage](Self::add_default_stages).
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
    pub fn add_startup_system_set(&mut self, system_set: SystemSet) -> &mut Self {
        self.add_startup_system_set_to_stage(StartupStage::Startup, system_set)
    }

    /// Adds a system to the [startup schedule](Self::add_default_stages), in the stage
    /// identified by `stage_label`.
    ///
    /// `stage_label` must refer to a stage inside the startup schedule.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # let mut app = App::new();
    /// # fn my_startup_system() {}
    /// #
    /// app.add_startup_system_to_stage(StartupStage::PreStartup, my_startup_system);
    /// ```
    pub fn add_startup_system_to_stage<Params>(
        &mut self,
        stage_label: impl StageLabel,
        system: impl IntoSystemDescriptor<Params>,
    ) -> &mut Self {
        self.schedule
            .stage(StartupSchedule, |schedule: &mut Schedule| {
                schedule.add_system_to_stage(stage_label, system)
            });
        self
    }

    /// Adds a [`SystemSet`] to the [startup schedule](Self::add_default_stages), in the stage
    /// identified by `stage_label`.
    ///
    /// `stage_label` must refer to a stage inside the startup schedule.
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
    /// app.add_startup_system_set_to_stage(
    ///     StartupStage::PreStartup,
    ///     SystemSet::new()
    ///         .with_system(startup_system_a)
    ///         .with_system(startup_system_b)
    ///         .with_system(startup_system_c),
    /// );
    /// ```
    pub fn add_startup_system_set_to_stage(
        &mut self,
        stage_label: impl StageLabel,
        system_set: SystemSet,
    ) -> &mut Self {
        self.schedule
            .stage(StartupSchedule, |schedule: &mut Schedule| {
                schedule.add_system_set_to_stage(stage_label, system_set)
            });
        self
    }

    /// Adds a new [`State`] with the given `initial` value.
    /// This inserts a new `State<T>` resource and adds a new "driver" to [`CoreStage::Update`].
    /// Each stage that uses `State<T>` for system run criteria needs a driver. If you need to use
    /// your state in a different stage, consider using [`Self::add_state_to_stage`] or manually
    /// adding [`State::get_driver`] to additional stages you need it in.
    pub fn add_state<T>(&mut self, initial: T) -> &mut Self
    where
        T: StateData,
    {
        self.add_state_to_stage(CoreStage::Update, initial)
    }

    /// Adds a new [`State`] with the given `initial` value.
    /// This inserts a new `State<T>` resource and adds a new "driver" to the given stage.
    /// Each stage that uses `State<T>` for system run criteria needs a driver. If you need to use
    /// your state in more than one stage, consider manually adding [`State::get_driver`] to the
    /// stages you need it in.
    pub fn add_state_to_stage<T>(&mut self, stage: impl StageLabel, initial: T) -> &mut Self
    where
        T: StateData,
    {
        self.insert_resource(State::new(initial))
            .add_system_set_to_stage(stage, State::<T>::get_driver())
    }

    /// Adds utility stages to the [`Schedule`], giving it a standardized structure.
    ///
    /// Adding those stages is necessary to make some core engine features work, like
    /// adding systems without specifying a stage, or registering events. This is however
    /// done by default by calling `App::default`, which is in turn called by
    /// [`App::new`].
    ///
    /// # The stages
    ///
    /// All the added stages, with the exception of the startup stage, run every time the
    /// schedule is invoked. The stages are the following, in order of execution:
    ///
    /// - **First:** Runs at the very start of the schedule execution cycle, even before the
    ///   startup stage.
    /// - **Startup:** This is actually a schedule containing sub-stages. Runs only once
    ///   when the app starts.
    ///     - **Pre-startup:** Intended for systems that need to run before other startup systems.
    ///     - **Startup:** The main startup stage. Startup systems are added here by default.
    ///     - **Post-startup:** Intended for systems that need to run after other startup systems.
    /// - **Pre-update:** Often used by plugins to prepare their internal state before the
    ///   update stage begins.
    /// - **Update:** Intended for user defined logic. Systems are added here by default.
    /// - **Post-update:** Often used by plugins to finalize their internal state after the
    ///   world changes that happened during the update stage.
    /// - **Last:** Runs right before the end of the schedule execution cycle.
    ///
    /// The labels for those stages are defined in the [`CoreStage`] and [`StartupStage`] `enum`s.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_app::prelude::*;
    /// #
    /// let app = App::empty().add_default_stages();
    /// ```
    pub fn add_default_stages(&mut self) -> &mut Self {
        self.add_stage(CoreStage::First, SystemStage::parallel())
            .add_stage(
                StartupSchedule,
                Schedule::default()
                    .with_run_criteria(ShouldRun::once)
                    .with_stage(StartupStage::PreStartup, SystemStage::parallel())
                    .with_stage(StartupStage::Startup, SystemStage::parallel())
                    .with_stage(StartupStage::PostStartup, SystemStage::parallel()),
            )
            .add_stage(CoreStage::PreUpdate, SystemStage::parallel())
            .add_stage(CoreStage::Update, SystemStage::parallel())
            .add_stage(CoreStage::PostUpdate, SystemStage::parallel())
            .add_stage(CoreStage::Last, SystemStage::parallel())
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
                .add_system_to_stage(CoreStage::First, Events::<T>::update_system);
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
                "Error adding plugin {}: : plugin was already added in application",
                plugin_name
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
        plugin.build(self);
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
    /// The provided function `f` is called by the [`update`](Self::update) method. The [`World`]
    /// parameter represents the main app world, while the [`App`] parameter is just a mutable
    /// reference to the `SubApp` itself.
    pub fn add_sub_app(
        &mut self,
        label: impl AppLabel,
        app: App,
        sub_app_runner: impl Fn(&mut World, &mut App) + 'static,
    ) -> &mut Self {
        self.sub_apps.insert(
            label.as_label(),
            SubApp {
                app,
                runner: Box::new(sub_app_runner),
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
}
