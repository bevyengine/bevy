use crate::{App, AppLabel, InternedAppLabel, Plugin, Plugins, PluginsState};
use alloc::{boxed::Box, string::String, vec::Vec};
use bevy_ecs::{
    message::MessageRegistry,
    prelude::*,
    schedule::{InternedScheduleLabel, InternedSystemSet, ScheduleBuildSettings, ScheduleLabel},
    system::{ScheduleSystem, SystemId, SystemInput},
};
use bevy_platform::collections::{HashMap, HashSet};
use core::fmt::Debug;

#[cfg(feature = "trace")]
use tracing::info_span;

type ExtractFn = Box<dyn FnMut(&mut World, &mut World) + Send>;

/// A secondary application with its own [`World`]. These can run independently of each other.
///
/// These are useful for situations where certain processes (e.g. a render thread) need to be kept
/// separate from the main application.
///
/// # Example
///
/// ```
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
/// // Create an app with a certain resource.
/// let mut app = App::new();
/// app.insert_resource(Val(10));
///
/// // Create a sub-app with the same resource and a single schedule.
/// let mut sub_app = SubApp::new();
/// sub_app.update_schedule = Some(Main.intern());
/// sub_app.insert_resource(Val(100));
///
/// // Setup an extract function to copy the resource's value in the main world.
/// sub_app.set_extract(|main_world, sub_world| {
///     sub_world.resource_mut::<Val>().0 = main_world.resource::<Val>().0;
/// });
///
/// // Schedule a system that will verify extraction is working.
/// sub_app.add_systems(Main, |counter: Res<Val>| {
///     // The value will be copied during extraction, so we should see 10 instead of 100.
///     assert_eq!(counter.0, 10);
/// });
///
/// // Add the sub-app to the main app.
/// app.insert_sub_app(ExampleApp, sub_app);
///
/// // Update the application once (using the default runner).
/// app.run();
/// ```
pub struct SubApp {
    /// The data of this application.
    world: World,
    /// List of plugins that have been added.
    pub(crate) plugin_registry: Vec<Box<dyn Plugin>>,
    /// The names of plugins that have been added to this app. (used to track duplicates and
    /// already-registered plugins)
    pub(crate) plugin_names: HashSet<String>,
    /// Panics if an update is attempted while plugins are building.
    pub(crate) plugin_build_depth: usize,
    pub(crate) plugins_state: PluginsState,
    /// The schedule that will be run by [`update`](Self::update).
    pub update_schedule: Option<InternedScheduleLabel>,
    /// A function that gives mutable access to two app worlds. This is primarily
    /// intended for copying data from the main world to secondary worlds.
    extract: Option<ExtractFn>,
}

impl Debug for SubApp {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "SubApp")
    }
}

impl Default for SubApp {
    fn default() -> Self {
        let mut world = World::new();
        world.init_resource::<Schedules>();
        Self {
            world,
            plugin_registry: Vec::default(),
            plugin_names: HashSet::default(),
            plugin_build_depth: 0,
            plugins_state: PluginsState::Adding,
            update_schedule: None,
            extract: None,
        }
    }
}

impl SubApp {
    /// Returns a default, empty [`SubApp`].
    pub fn new() -> Self {
        Self::default()
    }

    /// This method is a workaround. Each [`SubApp`] can have its own plugins, but [`Plugin`]
    /// works on an [`App`] as a whole.
    fn run_as_app<F>(&mut self, f: F)
    where
        F: FnOnce(&mut App),
    {
        let mut app = App::empty();
        core::mem::swap(self, &mut app.sub_apps.main);
        f(&mut app);
        core::mem::swap(self, &mut app.sub_apps.main);
    }

    /// Returns a reference to the [`World`].
    pub fn world(&self) -> &World {
        &self.world
    }

    /// Returns a mutable reference to the [`World`].
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
    }

    /// Runs the default schedule.
    ///
    /// Does not clear internal trackers used for change detection.
    pub fn run_default_schedule(&mut self) {
        if self.is_building_plugins() {
            panic!("SubApp::update() was called while a plugin was building.");
        }

        if let Some(label) = self.update_schedule {
            self.world.run_schedule(label);
        }
    }

    /// Runs the default schedule and updates internal component trackers.
    pub fn update(&mut self) {
        self.run_default_schedule();
        self.world.clear_trackers();
    }

    /// Extracts data from `world` into the app's world using the registered extract method.
    ///
    /// **Note:** There is no default extract method. Calling `extract` does nothing if
    /// [`set_extract`](Self::set_extract) has not been called.
    pub fn extract(&mut self, world: &mut World) {
        if let Some(f) = self.extract.as_mut() {
            f(world, &mut self.world);
        }
    }

    /// Sets the method that will be called by [`extract`](Self::extract).
    ///
    /// The first argument is the `World` to extract data from, the second argument is the app `World`.
    pub fn set_extract<F>(&mut self, extract: F) -> &mut Self
    where
        F: FnMut(&mut World, &mut World) + Send + 'static,
    {
        self.extract = Some(Box::new(extract));
        self
    }

    /// Take the function that will be called by [`extract`](Self::extract) out of the app, if any was set,
    /// and replace it with `None`.
    ///
    /// If you use Bevy, `bevy_render` will set a default extract function used to extract data from
    /// the main world into the render world as part of the Extract phase. In that case, you cannot replace
    /// it with your own function. Instead, take the Bevy default function with this, and install your own
    /// instead which calls the Bevy default.
    ///
    /// ```
    /// # use bevy_app::SubApp;
    /// # let mut app = SubApp::new();
    /// let mut default_fn = app.take_extract();
    /// app.set_extract(move |main, render| {
    ///     // Do pre-extract custom logic
    ///     // [...]
    ///
    ///     // Call Bevy's default, which executes the Extract phase
    ///     if let Some(f) = default_fn.as_mut() {
    ///         f(main, render);
    ///     }
    ///
    ///     // Do post-extract custom logic
    ///     // [...]
    /// });
    /// ```
    pub fn take_extract(&mut self) -> Option<ExtractFn> {
        self.extract.take()
    }

    /// See [`App::insert_resource`].
    pub fn insert_resource<R: Resource>(&mut self, resource: R) -> &mut Self {
        self.world.insert_resource(resource);
        self
    }

    /// See [`App::init_resource`].
    pub fn init_resource<R: Resource + FromWorld>(&mut self) -> &mut Self {
        self.world.init_resource::<R>();
        self
    }

    /// See [`App::add_systems`].
    pub fn add_systems<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
    ) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        schedules.add_systems(schedule, systems);

        self
    }

    /// See [`App::register_system`].
    pub fn register_system<I, O, M>(
        &mut self,
        system: impl IntoSystem<I, O, M> + 'static,
    ) -> SystemId<I, O>
    where
        I: SystemInput + 'static,
        O: 'static,
    {
        self.world.register_system(system)
    }

    /// See [`App::configure_sets`].
    #[track_caller]
    pub fn configure_sets<M>(
        &mut self,
        schedule: impl ScheduleLabel,
        sets: impl IntoScheduleConfigs<InternedSystemSet, M>,
    ) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        schedules.configure_sets(schedule, sets);
        self
    }

    /// See [`App::add_schedule`].
    pub fn add_schedule(&mut self, schedule: Schedule) -> &mut Self {
        let mut schedules = self.world.resource_mut::<Schedules>();
        schedules.insert(schedule);
        self
    }

    /// See [`App::init_schedule`].
    pub fn init_schedule(&mut self, label: impl ScheduleLabel) -> &mut Self {
        let label = label.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();
        if !schedules.contains(label) {
            schedules.insert(Schedule::new(label));
        }
        self
    }

    /// See [`App::get_schedule`].
    pub fn get_schedule(&self, label: impl ScheduleLabel) -> Option<&Schedule> {
        let schedules = self.world.get_resource::<Schedules>()?;
        schedules.get(label)
    }

    /// See [`App::get_schedule_mut`].
    pub fn get_schedule_mut(&mut self, label: impl ScheduleLabel) -> Option<&mut Schedule> {
        let schedules = self.world.get_resource_mut::<Schedules>()?;
        // We must call `.into_inner` here because the borrow checker only understands reborrows
        // using ordinary references, not our `Mut` smart pointers.
        schedules.into_inner().get_mut(label)
    }

    /// See [`App::edit_schedule`].
    pub fn edit_schedule(
        &mut self,
        label: impl ScheduleLabel,
        mut f: impl FnMut(&mut Schedule),
    ) -> &mut Self {
        let label = label.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();
        if !schedules.contains(label) {
            schedules.insert(Schedule::new(label));
        }

        let schedule = schedules.get_mut(label).unwrap();
        f(schedule);

        self
    }

    /// See [`App::configure_schedules`].
    pub fn configure_schedules(
        &mut self,
        schedule_build_settings: ScheduleBuildSettings,
    ) -> &mut Self {
        self.world_mut()
            .resource_mut::<Schedules>()
            .configure_schedules(schedule_build_settings);
        self
    }

    /// See [`App::allow_ambiguous_component`].
    pub fn allow_ambiguous_component<T: Component>(&mut self) -> &mut Self {
        self.world_mut().allow_ambiguous_component::<T>();
        self
    }

    /// See [`App::allow_ambiguous_resource`].
    pub fn allow_ambiguous_resource<T: Resource>(&mut self) -> &mut Self {
        self.world_mut().allow_ambiguous_resource::<T>();
        self
    }

    /// See [`App::ignore_ambiguity`].
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
        let schedule = schedule.intern();
        let mut schedules = self.world.resource_mut::<Schedules>();

        schedules.ignore_ambiguity(schedule, a, b);

        self
    }

    /// See [`App::add_message`].
    #[deprecated(since = "0.17.0", note = "Use `add_message` instead.")]
    pub fn add_event<T>(&mut self) -> &mut Self
    where
        T: Message,
    {
        self.add_message::<T>()
    }

    /// See [`App::add_message`].
    pub fn add_message<T>(&mut self) -> &mut Self
    where
        T: Message,
    {
        if !self.world.contains_resource::<Messages<T>>() {
            MessageRegistry::register_message::<T>(self.world_mut());
        }

        self
    }

    /// See [`App::add_plugins`].
    pub fn add_plugins<M>(&mut self, plugins: impl Plugins<M>) -> &mut Self {
        self.run_as_app(|app| plugins.add_to_app(app));
        self
    }

    /// See [`App::is_plugin_added`].
    pub fn is_plugin_added<T>(&self) -> bool
    where
        T: Plugin,
    {
        self.plugin_names.contains(core::any::type_name::<T>())
    }

    /// See [`App::get_added_plugins`].
    pub fn get_added_plugins<T>(&self) -> Vec<&T>
    where
        T: Plugin,
    {
        self.plugin_registry
            .iter()
            .filter_map(|p| p.downcast_ref())
            .collect()
    }

    /// Returns `true` if there is no plugin in the middle of being built.
    pub(crate) fn is_building_plugins(&self) -> bool {
        self.plugin_build_depth > 0
    }

    /// Return the state of plugins.
    #[inline]
    pub fn plugins_state(&mut self) -> PluginsState {
        match self.plugins_state {
            PluginsState::Adding => {
                let mut state = PluginsState::Ready;
                let plugins = core::mem::take(&mut self.plugin_registry);
                self.run_as_app(|app| {
                    for plugin in &plugins {
                        if !plugin.ready(app) {
                            state = PluginsState::Adding;
                            return;
                        }
                    }
                });
                self.plugin_registry = plugins;
                state
            }
            state => state,
        }
    }

    /// Runs [`Plugin::finish`] for each plugin.
    pub fn finish(&mut self) {
        // do hokey pokey with a boxed zst plugin (doesn't allocate)
        let mut hokeypokey: Box<dyn Plugin> = Box::new(crate::HokeyPokey);
        for i in 0..self.plugin_registry.len() {
            core::mem::swap(&mut self.plugin_registry[i], &mut hokeypokey);
            #[cfg(feature = "trace")]
            let _plugin_finish_span =
                info_span!("plugin finish", plugin = hokeypokey.name()).entered();
            self.run_as_app(|app| {
                hokeypokey.finish(app);
            });
            core::mem::swap(&mut self.plugin_registry[i], &mut hokeypokey);
        }
        self.plugins_state = PluginsState::Finished;
    }

    /// Runs [`Plugin::cleanup`] for each plugin.
    pub fn cleanup(&mut self) {
        // do hokey pokey with a boxed zst plugin (doesn't allocate)
        let mut hokeypokey: Box<dyn Plugin> = Box::new(crate::HokeyPokey);
        for i in 0..self.plugin_registry.len() {
            core::mem::swap(&mut self.plugin_registry[i], &mut hokeypokey);
            #[cfg(feature = "trace")]
            let _plugin_cleanup_span =
                info_span!("plugin cleanup", plugin = hokeypokey.name()).entered();
            self.run_as_app(|app| {
                hokeypokey.cleanup(app);
            });
            core::mem::swap(&mut self.plugin_registry[i], &mut hokeypokey);
        }
        self.plugins_state = PluginsState::Cleaned;
    }

    /// See [`App::register_type`].
    #[cfg(feature = "bevy_reflect")]
    pub fn register_type<T: bevy_reflect::GetTypeRegistration>(&mut self) -> &mut Self {
        let registry = self.world.resource_mut::<AppTypeRegistry>();
        registry.write().register::<T>();
        self
    }

    /// See [`App::register_type_data`].
    #[cfg(feature = "bevy_reflect")]
    pub fn register_type_data<
        T: bevy_reflect::Reflect + bevy_reflect::TypePath,
        D: bevy_reflect::TypeData + bevy_reflect::FromType<T>,
    >(
        &mut self,
    ) -> &mut Self {
        let registry = self.world.resource_mut::<AppTypeRegistry>();
        registry.write().register_type_data::<T, D>();
        self
    }

    /// See [`App::register_function`].
    #[cfg(feature = "reflect_functions")]
    pub fn register_function<F, Marker>(&mut self, function: F) -> &mut Self
    where
        F: bevy_reflect::func::IntoFunction<'static, Marker> + 'static,
    {
        let registry = self.world.resource_mut::<AppFunctionRegistry>();
        registry.write().register(function).unwrap();
        self
    }

    /// See [`App::register_function_with_name`].
    #[cfg(feature = "reflect_functions")]
    pub fn register_function_with_name<F, Marker>(
        &mut self,
        name: impl Into<alloc::borrow::Cow<'static, str>>,
        function: F,
    ) -> &mut Self
    where
        F: bevy_reflect::func::IntoFunction<'static, Marker> + 'static,
    {
        let registry = self.world.resource_mut::<AppFunctionRegistry>();
        registry.write().register_with_name(name, function).unwrap();
        self
    }
}

/// The collection of sub-apps that belong to an [`App`].
#[derive(Default)]
pub struct SubApps {
    /// The primary sub-app that contains the "main" world.
    pub main: SubApp,
    /// Other, labeled sub-apps.
    pub sub_apps: HashMap<InternedAppLabel, SubApp>,
}

impl SubApps {
    /// Calls [`update`](SubApp::update) for the main sub-app, and then calls
    /// [`extract`](SubApp::extract) and [`update`](SubApp::update) for the rest.
    pub fn update(&mut self) {
        #[cfg(feature = "trace")]
        let _bevy_update_span = info_span!("update").entered();
        {
            #[cfg(feature = "trace")]
            let _bevy_frame_update_span = info_span!("main app").entered();
            self.main.run_default_schedule();
        }
        for (_label, sub_app) in self.sub_apps.iter_mut() {
            #[cfg(feature = "trace")]
            let _sub_app_span = info_span!("sub app", name = ?_label).entered();
            sub_app.extract(&mut self.main.world);
            sub_app.update();
        }

        self.main.world.clear_trackers();
    }

    /// Returns an iterator over the sub-apps (starting with the main one).
    pub fn iter(&self) -> impl Iterator<Item = &SubApp> + '_ {
        core::iter::once(&self.main).chain(self.sub_apps.values())
    }

    /// Returns a mutable iterator over the sub-apps (starting with the main one).
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut SubApp> + '_ {
        core::iter::once(&mut self.main).chain(self.sub_apps.values_mut())
    }

    /// Extract data from the main world into the [`SubApp`] with the given label and perform an update if it exists.
    pub fn update_subapp_by_label(&mut self, label: impl AppLabel) {
        if let Some(sub_app) = self.sub_apps.get_mut(&label.intern()) {
            sub_app.extract(&mut self.main.world);
            sub_app.update();
        }
    }
}
