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
        app::App, DynamicPlugin, First, FixedUpdate, Last, Main, Plugin, PluginGroup, PostStartup,
        PostUpdate, PreStartup, PreUpdate, Startup, StateTransition, Update,
    };
}

use bevy_ecs::{
    schedule::ScheduleLabel,
    system::{Local, Resource},
    world::{Mut, World},
};

/// The schedule that contains the app logic that is evaluated each tick of [`App::update()`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Main;

/// The schedule that runs before [`Startup`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreStartup;

/// The schedule that runs once when the app starts.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Startup;

/// The schedule that runs once after [`Startup`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostStartup;

/// The schedule that runs once when the app starts.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct First;

/// The schedule that contains work required to make [`Update`] logic.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreUpdate;

/// Runs state transitions.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateTransition;

/// Runs the [`FixedUpdate`] schedule in a loop according until all relevant elapsed time has been "consumed".
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedUpdateLoop;

/// The schedule that contains systems which only run after a fixed period of time has elapsed.
///
/// The exclusive `run_fixed_update_schedule` system runs this schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedUpdate;

/// The schedule that contains app logic.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update;

/// The schedule that contains systems that respond to [`Update`].
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostUpdate;

/// Runs last in the schedule
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Last;

/// Defines the schedules to be run for the [`Main`] schedule, including
/// their order.
#[derive(Resource, Debug)]
pub struct MainScheduleOrder {
    labels: Vec<Box<dyn ScheduleLabel>>,
}

impl Default for MainScheduleOrder {
    fn default() -> Self {
        Self {
            labels: vec![
                Box::new(First),
                Box::new(PreUpdate),
                Box::new(StateTransition),
                Box::new(FixedUpdateLoop),
                Box::new(Update),
                Box::new(PostUpdate),
                Box::new(Last),
            ],
        }
    }
}

impl MainScheduleOrder {
    /// Adds the given `schedule` after the `after` schedule
    pub fn insert_after(&mut self, after: impl ScheduleLabel, schedule: impl ScheduleLabel) {
        let index = self
            .labels
            .iter()
            .position(|current| (**current).eq(&after))
            .unwrap_or_else(|| panic!("Expected {after:?} to exist"));
        self.labels.insert(index + 1, Box::new(schedule));
    }
}

impl Main {
    /// A system that runs the "main schedule"
    pub fn run_main(world: &mut World, mut run_at_least_once: Local<bool>) {
        if !*run_at_least_once {
            world.run_schedule(PreStartup);
            world.run_schedule(Startup);
            world.run_schedule(PostStartup);
            *run_at_least_once = true;
        }

        world.resource_scope(|world, order: Mut<MainScheduleOrder>| {
            for label in &order.labels {
                world.run_schedule_ref(&**label);
            }
        });
    }

    /// Initializes the [`Main`] schedule and sub-schedules on the given `app`.
    pub fn init(app: &mut App) {
        app.init_schedule(Main)
            .init_schedule(PreStartup)
            .init_schedule(Startup)
            .init_schedule(PostStartup)
            .init_schedule(First)
            .init_schedule(PreUpdate)
            .init_schedule(StateTransition)
            .init_schedule(FixedUpdateLoop)
            .init_schedule(FixedUpdate)
            .init_schedule(Update)
            .init_schedule(PostUpdate)
            .init_schedule(Last)
            .init_resource::<MainScheduleOrder>()
            .add_systems(Main, Self::run_main);
    }
}
