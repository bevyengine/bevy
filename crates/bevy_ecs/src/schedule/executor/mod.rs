mod multi_threaded;
mod simple;
mod single_threaded;

pub use self::multi_threaded::{MainThreadExecutor, MultiThreadedExecutor};
pub use self::simple::SimpleExecutor;
pub use self::single_threaded::SingleThreadedExecutor;

use fixedbitset::FixedBitSet;

use crate::{
    schedule::{BoxedCondition, NodeId},
    system::BoxedSystem,
    world::World,
};

/// Types that can run a [`SystemSchedule`] on a [`World`].
pub(super) trait SystemExecutor: Send + Sync {
    fn kind(&self) -> ExecutorKind;
    fn init(&mut self, schedule: &SystemSchedule);
    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World);
    fn set_apply_final_buffers(&mut self, value: bool);
}

/// Specifies how a [`Schedule`](super::Schedule) will be run.
///
/// The default depends on the target platform:
///  - [`SingleThreaded`](ExecutorKind::SingleThreaded) on WASM.
///  - [`MultiThreaded`](ExecutorKind::MultiThreaded) everywhere else.
#[derive(PartialEq, Eq, Default)]
pub enum ExecutorKind {
    /// Runs the schedule using a single thread.
    ///
    /// Useful if you're dealing with a single-threaded environment, saving your threads for
    /// other things, or just trying minimize overhead.
    #[cfg_attr(target_arch = "wasm32", default)]
    SingleThreaded,
    /// Like [`SingleThreaded`](ExecutorKind::SingleThreaded) but calls [`apply_buffers`](crate::system::System::apply_buffers)
    /// immediately after running each system.
    Simple,
    /// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
    #[cfg_attr(not(target_arch = "wasm32"), default)]
    MultiThreaded,
}

/// Holds systems and conditions of a [`Schedule`](super::Schedule) sorted in topological order
/// (along with dependency information for multi-threaded execution).
///
/// Since the arrays are sorted in the same order, elements are referenced by their index.
/// `FixedBitSet` is used as a smaller, more efficient substitute of `HashSet<usize>`.
#[derive(Default)]
pub struct SystemSchedule {
    pub(super) systems: Vec<BoxedSystem>,
    pub(super) system_conditions: Vec<Vec<BoxedCondition>>,
    pub(super) set_conditions: Vec<Vec<BoxedCondition>>,
    pub(super) system_ids: Vec<NodeId>,
    pub(super) set_ids: Vec<NodeId>,
    pub(super) system_dependencies: Vec<usize>,
    pub(super) system_dependents: Vec<Vec<usize>>,
    pub(super) sets_with_conditions_of_systems: Vec<FixedBitSet>,
    pub(super) systems_in_sets_with_conditions: Vec<FixedBitSet>,
    pub(super) systems_with_stepping_enabled: FixedBitSet,
    pub(super) step_state: StepState,
}

impl SystemSchedule {
    pub const fn new() -> Self {
        Self {
            systems: Vec::new(),
            system_conditions: Vec::new(),
            set_conditions: Vec::new(),
            system_ids: Vec::new(),
            set_ids: Vec::new(),
            system_dependencies: Vec::new(),
            system_dependents: Vec::new(),
            sets_with_conditions_of_systems: Vec::new(),
            systems_in_sets_with_conditions: Vec::new(),
            systems_with_stepping_enabled: FixedBitSet::new(),
            step_state: StepState::RunAll,
        }
    }

    /// Return the index of the next system to be run if this schedule is
    /// stepped
    pub fn stepping_next_system_index(&self) -> Option<usize> {
        let index = match self.step_state {
            StepState::RunAll => None,
            StepState::Wait(i) | StepState::Next(i) | StepState::Remaining(i) => Some(i),
        }?;

        self.find_stepping_system(index)
    }

    /// This method returns the list of systems to be skipped when the
    /// system executor runs, and updates `step_state` to prepare for the
    /// next call.
    pub fn step(&mut self) -> Option<FixedBitSet> {
        match self.step_state {
            StepState::RunAll => None,
            StepState::Wait(_) => Some(self.systems_with_stepping_enabled.clone()),
            StepState::Next(index) => {
                let next = self.find_stepping_system(index)?;

                // clone the list of stepping systems, then disable
                let mut mask = self.systems_with_stepping_enabled.clone();
                mask.toggle(next);

                // Transition to the wait state. it's safe to set the value
                // beyond the number of systems in the schedule.  All uses of
                // that value use `find_stepping_system`, which will wrap it
                // to a safe value.
                self.step_state = StepState::Wait(next + 1);

                Some(mask)
            }
            StepState::Remaining(index) => {
                let next = self.find_stepping_system(index)?;

                // We need to mark all systems that observe stepping prior
                // to `next` as completed.  We do this in three steps:
                //
                // 1. set the bit for every system below `next` in a bitset
                // 2. clear the bits for every system ignoring stepping;
                //    we do this with a bitwise AND between the mask and
                //    those systems that are observing stepping
                // 3. We set those bits in the completed_systems bitmask by
                //    using a bitwise OR.
                //
                let mut mask = FixedBitSet::with_capacity(self.systems.len());
                mask.insert_range(0..next);
                mask &= &self.systems_with_stepping_enabled;

                // transition to wait state, starting at the first system
                self.step_state = StepState::Wait(0);

                Some(mask)
            }
        }
    }

    /// starting at system index `start`, return the index of the first system
    /// that has stepping enabled.
    fn find_stepping_system(&self, start: usize) -> Option<usize> {
        for i in start..self.systems_with_stepping_enabled.len() {
            if self.systems_with_stepping_enabled[i] {
                return Some(i);
            }
        }
        (0..start).find(|i| self.systems_with_stepping_enabled[*i])
    }
}

/// Instructs the executor to call [`apply_buffers`](crate::system::System::apply_buffers)
/// on the systems that have run but not applied their buffers.
///
/// **Notes**
/// - This function (currently) does nothing if it's called manually or wrapped inside a [`PipeSystem`](crate::system::PipeSystem).
/// - Modifying a [`Schedule`](super::Schedule) may change the order buffers are applied.
#[allow(unused_variables)]
pub fn apply_system_buffers(world: &mut World) {}

/// Returns `true` if the [`System`](crate::system::System) is an instance of [`apply_system_buffers`].
pub(super) fn is_apply_system_buffers(system: &BoxedSystem) -> bool {
    use std::any::Any;
    // deref to use `System::type_id` instead of `Any::type_id`
    system.as_ref().type_id() == apply_system_buffers.type_id()
}

/// Stepping state, stored in [`SystemSchedule`], used by [`SystemExecutor`] to
/// determine which systems in the schedule should be run.
#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub(super) enum StepState {
    /// Run only systems that are ignoring stepping;
    /// see [`ignore_stepping`](`super::IntoSystemConfigs::ignore_stepping`)
    Wait(usize),
    /// Run the next system in the schedule that does not ignore stepping, ,
    /// along with all systems that [`ignore
    /// stepping`](`super::IntoSystemConfigs::ignore_stepping`).
    Next(usize),

    /// Run all remaining systems in the schedule that have not yet been run,
    /// along with all systems that
    /// [`ignore stepping`](`super::IntoSystemConfigs::ignore_stepping`).
    Remaining(usize),

    /// Stepping is disabled; run all systems.
    #[default]
    RunAll,
}
