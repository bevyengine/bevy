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
    scheduling::{Schedule, ScheduleLabel, SystemSet},
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
    /// This schedule is run during [`CoreSet::FixedTimestep`] via an exclusive system, between [`CoreSet::First`] and [`CoreSet::PreUpdate`]
    FixedTimestep,
}

impl CoreSchedule {
    /// An exclusive system that controls which schedule should be running.
    ///
    /// [`CoreSchedule::Main`] is always run.
    ///
    /// If this is the first time this system has been run, [`CoreSchedule::Startup`] will run before [`CoreSchedule::Main`].
    pub fn outer_loop(world: &mut World, mut run_at_least_once: Local<bool>) {
        if !*run_at_least_once {
            world.run_schedule(&CoreSchedule::Startup);
            *run_at_least_once = true;
        }

        world.run_schedule(&CoreSchedule::Main);
    }

    /// Initializes a schedule for [`CoreSchedule::Outer`] that contains the [`outer_loop`] system.
    pub fn outer_schedule() -> Schedule {
        let mut schedule = Schedule::new();
        schedule.add_system(Self::outer_loop);
        schedule
    }
}

/// The names of the default [`App`] system sets.
///
/// These are ordered in the same order they are listed.
///
/// The corresponding [`SystemSets`](bevy_ecs::schedule::SystemSet) are added by [`App::add_default_sets`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum CoreSet {
    /// Runs before all other members of this set.
    First,
    /// Runs systems that should only occur after a fixed period of time.
    ///
    /// The `fixed_timestep` system runs the [`CoreSchedule::FixedTimestep`] system in this system set.
    FixedTimestep,
    /// Runs before [`CoreSet::Update`].
    PreUpdate,
    /// Applies [`State`](bevy_ecs::schedule::State) transitions
    StateTransitions,
    /// Responsible for doing most app logic. Systems should be registered here by default.
    Update,
    /// Runs after [`CoreSet::Update`].
    PostUpdate,
    /// Runs after all other members of this set.
    Last,
}

/// The names of the default [`App`] startup sets, which live in [`CoreSchedule::Startup`].
///
/// The corresponding [`SystemSets`](bevy_ecs::schedule::SystemSet) are added by [`App::add_default_sets`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum StartupSet {
    /// Runs once before [`StartupSet::Startup`].
    PreStartup,
    /// Runs once when an [`App`] starts up.
    Startup,
    /// Runs once after [`StartupSet::Startup`].
    PostStartup,
}
