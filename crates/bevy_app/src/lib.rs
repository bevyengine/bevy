//! This crate is about everything concerning the highest-level, application layer of a Bevy app.

#![warn(missing_docs)]

mod app;
mod plugin;
mod plugin_group;
mod schedule_runner;

#[cfg(feature = "bevy_ci_testing")]
mod ci_testing;

pub use app::*;
pub use bevy_derive::DynamicPlugin;
pub use plugin::*;
pub use plugin_group::*;
pub use schedule_runner::*;

#[allow(missing_docs)]
pub mod prelude {
    #[cfg(feature = "bevy_reflect")]
    #[doc(hidden)]
    pub use crate::AppTypeRegistry;
    #[doc(hidden)]
    pub use crate::{
        app::App, CoreSchedule, CoreSet, DynamicPlugin, Plugin, PluginGroup, StartupSet,
    };
}

use bevy_ecs::{
    schedule::{
        apply_system_buffers, IntoSystemConfig, IntoSystemSetConfig, IntoSystemSetConfigs,
        Schedule, ScheduleLabel, SystemSet,
    },
    system::Local,
    world::World,
};

/// The names of the default [`App`] schedules.
///
/// The corresponding [`Schedule`](bevy_ecs::schedule::Schedule) objects are added by [`App::add_default_schedules`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub enum CoreSchedule {
    /// The schedule that runs once when the app starts.
    Startup,
    /// The schedule that contains the app logic that is evaluated each tick of [`App::update()`].
    Main,
    /// The schedule that controls which schedules run.
    ///
    /// This is typically created using the [`CoreSchedule::outer_schedule`] method,
    /// and does not need to manipulated during ordinary use.
    Outer,
    /// The schedule that contains systems which only run after a fixed period of time has elapsed.
    ///
    /// The exclusive `run_fixed_update_schedule` system runs this schedule during the [`CoreSet::FixedUpdate`] system set.
    FixedUpdate,
}

impl CoreSchedule {
    /// An exclusive system that controls which schedule should be running.
    ///
    /// [`CoreSchedule::Main`] is always run.
    ///
    /// If this is the first time this system has been run, [`CoreSchedule::Startup`] will run before [`CoreSchedule::Main`].
    pub fn outer_loop(world: &mut World, mut run_at_least_once: Local<bool>) {
        if !*run_at_least_once {
            world.run_schedule(CoreSchedule::Startup);
            *run_at_least_once = true;
        }

        world.run_schedule(CoreSchedule::Main);
    }

    /// Initializes a single threaded schedule for [`CoreSchedule::Outer`] that contains the [`outer_loop`](CoreSchedule::outer_loop) system.
    pub fn outer_schedule() -> Schedule {
        let mut schedule = Schedule::new();
        schedule.set_executor_kind(bevy_ecs::schedule::ExecutorKind::SingleThreaded);
        schedule.add_system(Self::outer_loop);
        schedule
    }
}

/// The names of the default [`App`] system sets.
///
/// These are ordered in the same order they are listed.
///
/// The corresponding [`SystemSets`](bevy_ecs::schedule::SystemSet) are added by [`App::add_default_schedules`].
///
/// The `*Flush` sets are assigned to the copy of [`apply_system_buffers`]
/// that runs immediately after the matching system set.
/// These can be useful for ordering, but you almost never want to add your systems to these sets.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
#[system_set(base)]
pub enum CoreSet {
    /// Runs before all other members of this set.
    First,
    /// The copy of [`apply_system_buffers`] that runs immediately after `First`.
    FirstFlush,
    /// Runs before [`CoreSet::Update`].
    PreUpdate,
    /// The copy of [`apply_system_buffers`] that runs immediately after `PreUpdate`.
    PreUpdateFlush,
    /// Applies [`State`](bevy_ecs::schedule::State) transitions
    StateTransitions,
    /// Runs systems that should only occur after a fixed period of time.
    ///
    /// The `run_fixed_update_schedule` system runs the [`CoreSchedule::FixedUpdate`] system in this system set.
    FixedUpdate,
    /// Responsible for doing most app logic. Systems should be registered here by default.
    Update,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Update`.
    UpdateFlush,
    /// Runs after [`CoreSet::Update`].
    PostUpdate,
    /// The copy of [`apply_system_buffers`] that runs immediately after `PostUpdate`.
    PostUpdateFlush,
    /// Runs after all other members of this set.
    Last,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Last`.
    LastFlush,
}

impl CoreSet {
    /// Sets up the base structure of [`CoreSchedule::Main`].
    ///
    /// The sets defined in this enum are configured to run in order,
    /// and a copy of [`apply_system_buffers`] is inserted at each `*Flush` label.
    pub fn base_schedule() -> Schedule {
        use CoreSet::*;
        let mut schedule = Schedule::new();

        // Create "stage-like" structure using buffer flushes + ordering
        schedule
            .set_default_base_set(Update)
            .add_system(apply_system_buffers.in_base_set(FirstFlush))
            .add_system(apply_system_buffers.in_base_set(PreUpdateFlush))
            .add_system(apply_system_buffers.in_base_set(UpdateFlush))
            .add_system(apply_system_buffers.in_base_set(PostUpdateFlush))
            .add_system(apply_system_buffers.in_base_set(LastFlush))
            .configure_sets(
                (
                    First,
                    FirstFlush,
                    PreUpdate,
                    PreUpdateFlush,
                    StateTransitions,
                    FixedUpdate,
                    Update,
                    UpdateFlush,
                    PostUpdate,
                    PostUpdateFlush,
                    Last,
                    LastFlush,
                )
                    .chain(),
            );
        schedule
    }
}

/// The names of the default [`App`] startup sets, which live in [`CoreSchedule::Startup`].
///
/// The corresponding [`SystemSets`](bevy_ecs::schedule::SystemSet) are added by [`App::add_default_schedules`].
///
/// The `*Flush` sets are assigned to the copy of [`apply_system_buffers`]
/// that runs immediately after the matching system set.
/// These can be useful for ordering, but you almost never want to add your systems to these sets.
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum StartupSet {
    /// Runs once before [`StartupSet::Startup`].
    PreStartup,
    /// The copy of [`apply_system_buffers`] that runs immediately after `PreStartup`.
    PreStartupFlush,
    /// Runs once when an [`App`] starts up.
    Startup,
    /// The copy of [`apply_system_buffers`] that runs immediately after `Startup`.
    StartupFlush,
    /// Runs once after [`StartupSet::Startup`].
    PostStartup,
    /// The copy of [`apply_system_buffers`] that runs immediately after `PostStartup`.
    PostStartupFlush,
}

impl StartupSet {
    /// Sets up the base structure of [`CoreSchedule::Startup`].
    ///
    /// The sets defined in this enum are configured to run in order,
    /// and a copy of [`apply_system_buffers`] is inserted at each `*Flush` label.
    pub fn base_schedule() -> Schedule {
        use StartupSet::*;
        let mut schedule = Schedule::new();

        // Create "stage-like" structure using buffer flushes + ordering
        schedule.add_system(apply_system_buffers.in_set(PreStartupFlush));
        schedule.add_system(apply_system_buffers.in_set(StartupFlush));
        schedule.add_system(apply_system_buffers.in_set(PostStartupFlush));

        schedule.configure_set(PreStartup.before(PreStartupFlush));
        schedule.configure_set(Startup.after(PreStartupFlush).before(StartupFlush));
        schedule.configure_set(PostStartup.after(StartupFlush).before(PostStartupFlush));

        schedule
    }
}
