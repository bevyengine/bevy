use crate::{App, Plugin};
use bevy_ecs::{
    schedule::{ExecutorKind, Schedule, ScheduleLabel},
    system::{Local, Resource},
    world::{Mut, World},
};

/// The schedule that contains the app logic that is evaluated each tick of [`App::update()`].
///
/// By default, it will run the following schedules in the given order:
///
/// On the first run of the schedule (and only on the first run), it will run:
/// * [`PreStartup`]
/// * [`Startup`]
/// * [`PostStartup`]
///
/// Then it will run:
/// * [`First`]
/// * [`PreUpdate`]
/// * [`StateTransition`]
/// * [`RunFixedUpdateLoop`]
///     * This will run [`FixedUpdate`] zero to many times, based on how much time has elapsed.
/// * [`Update`]
/// * [`PostUpdate`]
/// * [`Last`]
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Main;

/// The schedule that runs before [`Startup`].
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreStartup;

/// The schedule that runs once when the app starts.
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Startup;

/// The schedule that runs once after [`Startup`].
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostStartup;

/// Runs first in the schedule.
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct First;

/// The schedule that contains logic that must run before [`Update`]. For example, a system that reads raw keyboard
/// input OS events into an `Events` resource. This enables systems in [`Update`] to consume the events from the `Events`
/// resource without actually knowing about (or taking a direct scheduler dependency on) the "os-level keyboard event system".
///
/// [`PreUpdate`] exists to do "engine/plugin preparation work" that ensures the APIs consumed in [`Update`] are "ready".
/// [`PreUpdate`] abstracts out "pre work implementation details".
///
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PreUpdate;

/// Runs [state transitions](bevy_ecs::schedule::States).
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct StateTransition;

/// Runs the [`FixedUpdate`] schedule in a loop according until all relevant elapsed time has been "consumed".
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct RunFixedUpdateLoop;

/// The schedule that contains systems which only run after a fixed period of time has elapsed.
///
/// The exclusive `run_fixed_update_schedule` system runs this schedule.
/// This is run by the [`RunFixedUpdateLoop`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FixedUpdate;

/// The schedule that contains app logic.
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update;

/// The schedule that contains scene spawning.
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SpawnScene;

/// The schedule that contains logic that must run after [`Update`]. For example, synchronizing "local transforms" in a hierarchy
/// to "global" absolute transforms. This enables the [`PostUpdate`] transform-sync system to react to "local transform" changes in
/// [`Update`] without the [`Update`] systems needing to know about (or add scheduler dependencies for) the "global transform sync system".
///
/// [`PostUpdate`] exists to do "engine/plugin response work" to things that happened in [`Update`].
/// [`PostUpdate`] abstracts out "implementation details" from users defining systems in [`Update`].
///
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct PostUpdate;

/// Runs last in the schedule.
/// This is run by the [`Main`] schedule.
#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
pub struct Last;

/// Defines the schedules to be run for the [`Main`] schedule, including
/// their order.
#[derive(Resource, Debug)]
pub struct MainScheduleOrder {
    /// The labels to run for the [`Main`] schedule (in the order they will be run).
    pub labels: Vec<Box<dyn ScheduleLabel>>,
}

impl Default for MainScheduleOrder {
    fn default() -> Self {
        Self {
            labels: vec![
                Box::new(First),
                Box::new(PreUpdate),
                Box::new(StateTransition),
                Box::new(RunFixedUpdateLoop),
                Box::new(Update),
                Box::new(SpawnScene),
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
            let _ = world.try_run_schedule(PreStartup);
            let _ = world.try_run_schedule(Startup);
            let _ = world.try_run_schedule(PostStartup);
            *run_at_least_once = true;
        }

        world.resource_scope(|world, order: Mut<MainScheduleOrder>| {
            for label in &order.labels {
                let _ = world.try_run_schedule(&**label);
            }
        });
    }
}

/// Initializes the [`Main`] schedule, sub schedules,  and resources for a given [`App`].
pub struct MainSchedulePlugin;

impl Plugin for MainSchedulePlugin {
    fn build(&self, app: &mut App) {
        // simple "facilitator" schedules benefit from simpler single threaded scheduling
        let mut main_schedule = Schedule::new(Main);
        main_schedule.set_executor_kind(ExecutorKind::SingleThreaded);
        let mut fixed_update_loop_schedule = Schedule::new(RunFixedUpdateLoop);
        fixed_update_loop_schedule.set_executor_kind(ExecutorKind::SingleThreaded);

        app.add_schedule(main_schedule)
            .add_schedule(fixed_update_loop_schedule)
            .init_resource::<MainScheduleOrder>()
            .add_systems(Main, Main::run_main);
    }
}
