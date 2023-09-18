#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use fixedbitset::FixedBitSet;
use std::panic::AssertUnwindSafe;

use crate::{
    schedule::{BoxedCondition, ExecutorKind, SystemExecutor, SystemSchedule},
    world::World,
};

/// A variant of [`SingleThreadedExecutor`](crate::schedule::SingleThreadedExecutor) that calls
/// [`apply_deferred`](crate::system::System::apply_deferred) immediately after running each system.
#[derive(Default)]
pub struct SimpleExecutor {
    /// Systems sets whose conditions have been evaluated.
    evaluated_sets: FixedBitSet,
    /// Systems that have run or been skipped.
    completed_systems: FixedBitSet,
}

impl SystemExecutor for SimpleExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::Simple
    }

    fn set_apply_final_deferred(&mut self, _: bool) {
        // do nothing. simple executor does not do a final sync
    }

    fn init(&mut self, schedule: &SystemSchedule) {
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();
        self.evaluated_sets = FixedBitSet::with_capacity(set_count);
        self.completed_systems = FixedBitSet::with_capacity(sys_count);
    }

    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
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
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                system.run((), world);
            }));
            if let Err(payload) = res {
                eprintln!("Encountered a panic in system `{}`!", &*system.name());
                std::panic::resume_unwind(payload);
            }

            system.apply_deferred(world);
        }

        self.evaluated_sets.clear();
        self.completed_systems.clear();
    }
}

impl SimpleExecutor {
    /// Creates a new simple executor for use in a [`Schedule`](crate::schedule::Schedule).
    /// This calls each system in order and immediately calls [`System::apply_deferred`](crate::system::System::apply_deferred).
    pub const fn new() -> Self {
        Self {
            evaluated_sets: FixedBitSet::new(),
            completed_systems: FixedBitSet::new(),
        }
    }
}

fn evaluate_and_fold_conditions(conditions: &mut [BoxedCondition], world: &mut World) -> bool {
    // not short-circuiting is intentional
    #[allow(clippy::unnecessary_fold)]
    conditions
        .iter_mut()
        .map(|condition| condition.run((), world))
        .fold(true, |acc, res| acc && res)
}
