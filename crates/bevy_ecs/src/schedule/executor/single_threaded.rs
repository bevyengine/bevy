#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use fixedbitset::FixedBitSet;

use super::StepState;
use crate::{
    schedule::{
        is_apply_system_buffers, BoxedCondition, ExecutorKind, SystemExecutor, SystemSchedule,
    },
    world::World,
};

/// Runs the schedule using a single thread.
///
/// Useful if you're dealing with a single-threaded environment, saving your threads for
/// other things, or just trying minimize overhead.
#[derive(Default)]
pub struct SingleThreadedExecutor {
    /// System sets whose conditions have been evaluated.
    evaluated_sets: FixedBitSet,
    /// Systems that have run or been skipped.
    completed_systems: FixedBitSet,
    /// Systems that have run but have not had their buffers applied.
    unapplied_systems: FixedBitSet,
    /// Setting when true applies system buffers after all systems have run
    apply_final_buffers: bool,
    /// Storage for step state
    step_state: StepState,
}

impl SystemExecutor for SingleThreadedExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::SingleThreaded
    }

    fn set_apply_final_buffers(&mut self, apply_final_buffers: bool) {
        self.apply_final_buffers = apply_final_buffers;
    }

    fn init(&mut self, schedule: &SystemSchedule) {
        // pre-allocate space
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();
        self.evaluated_sets = FixedBitSet::with_capacity(set_count);
        self.completed_systems = FixedBitSet::with_capacity(sys_count);
        self.unapplied_systems = FixedBitSet::with_capacity(sys_count);
    }

    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        /// return the next system not exempt from stepping starting at, and
        /// inclusive of, first.  If there are no more systems that can be
        /// stepped, the first non-exempt system index will be returned.
        fn next_system(schedule: &SystemSchedule, first: usize) -> usize {
            for i in first..schedule.systems_with_stepping.len() {
                if schedule.systems_with_stepping[i] {
                    return i;
                }
            }
            for i in 0..first {
                if schedule.systems_with_stepping[i] {
                    return i;
                }
            }
            panic!("all systems exempt from stepping");
        }

        match self.step_state {
            StepState::RunAll => (),
            StepState::Wait(_) => {
                self.completed_systems |= &schedule.systems_with_stepping;
            }
            StepState::Next(next) => {
                let next = next_system(schedule, next);
                assert!(schedule.systems_with_stepping[next]);

                self.completed_systems |= &schedule.systems_with_stepping;
                self.completed_systems.toggle(next);

                self.step_state = StepState::Wait(next + 1);
            }
            StepState::Remaining(next) => {
                let next = next_system(schedule, next);
                let mut mask = FixedBitSet::with_capacity(schedule.systems.len());
                mask.insert_range(0..next);
                mask &= &schedule.systems_with_stepping;
                self.completed_systems |= mask;
                self.step_state = StepState::Wait(0);
            }
        }

        for system_index in 0..schedule.systems.len() {
            #[cfg(feature = "trace")]
            let name = schedule.systems[system_index].name();
            #[cfg(feature = "trace")]
            let should_run_span = info_span!("check_conditions", name = &*name).entered();

            let mut should_run = !self.completed_systems.contains(system_index);

            for set_idx in schedule.sets_with_conditions_of_systems[system_index].ones() {
                if self.evaluated_sets.contains(set_idx) {
                    continue;
                }

                // evaluate system set's conditions
                let set_conditions_met =
                    evaluate_and_fold_conditions(&mut schedule.set_conditions[set_idx], world);

                if !set_conditions_met {
                    self.completed_systems
                        .union_with(&schedule.systems_in_sets_with_conditions[set_idx]);
                }

                should_run &= set_conditions_met;
                self.evaluated_sets.insert(set_idx);
            }

            // evaluate system's conditions
            let system_conditions_met =
                evaluate_and_fold_conditions(&mut schedule.system_conditions[system_index], world);

            should_run &= system_conditions_met;

            #[cfg(feature = "trace")]
            should_run_span.exit();

            // system has either been skipped or will run
            self.completed_systems.insert(system_index);

            if !should_run {
                continue;
            }

            let system = &mut schedule.systems[system_index];
            if is_apply_system_buffers(system) {
                #[cfg(feature = "trace")]
                let system_span = info_span!("system", name = &*name).entered();
                self.apply_system_buffers(schedule, world);
                #[cfg(feature = "trace")]
                system_span.exit();
            } else {
                #[cfg(feature = "trace")]
                let system_span = info_span!("system", name = &*name).entered();
                system.run((), world);
                #[cfg(feature = "trace")]
                system_span.exit();
                self.unapplied_systems.insert(system_index);
            }
        }

        if self.apply_final_buffers {
            self.apply_system_buffers(schedule, world);
        }
        self.evaluated_sets.clear();
        self.completed_systems.clear();
    }

    fn stepping(&self) -> bool {
        self.step_state != StepState::RunAll
    }

    fn next_system(&self) -> Option<usize> {
        match self.step_state {
            StepState::Wait(next) | StepState::Next(next) | StepState::Remaining(next) => {
                if next >= self.completed_systems.len() {
                    Some(0)
                } else {
                    Some(next)
                }
            }
            StepState::RunAll => None,
        }
    }

    fn set_stepping(&mut self, stepping: bool) {
        self.step_state = match stepping {
            true => StepState::Wait(0),
            false => StepState::RunAll,
        }
    }

    fn step_system(&mut self) {
        if let StepState::Wait(next) = self.step_state {
            self.step_state = StepState::Next(next);
        }
    }

    fn step_frame(&mut self) {
        if let StepState::Wait(next) = self.step_state {
            self.step_state = StepState::Remaining(next);
        }
    }
}

impl SingleThreadedExecutor {
    pub const fn new() -> Self {
        Self {
            evaluated_sets: FixedBitSet::new(),
            completed_systems: FixedBitSet::new(),
            unapplied_systems: FixedBitSet::new(),
            apply_final_buffers: true,
            step_state: StepState::RunAll,
        }
    }

    fn apply_system_buffers(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        for system_index in self.unapplied_systems.ones() {
            let system = &mut schedule.systems[system_index];
            system.apply_buffers(world);
        }

        self.unapplied_systems.clear();
    }
}

fn evaluate_and_fold_conditions(conditions: &mut [BoxedCondition], world: &mut World) -> bool {
    // not short-circuiting is intentional
    #[allow(clippy::unnecessary_fold)]
    conditions
        .iter_mut()
        .map(|condition| {
            #[cfg(feature = "trace")]
            let _condition_span = info_span!("condition", name = &*condition.name()).entered();
            condition.run((), world)
        })
        .fold(true, |acc, res| acc && res)
}
