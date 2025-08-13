#![expect(deprecated, reason = "Everything here is deprecated")]

use core::panic::AssertUnwindSafe;
use fixedbitset::FixedBitSet;

#[cfg(feature = "trace")]
use tracing::info_span;

#[cfg(feature = "std")]
use std::eprintln;

#[cfg(feature = "hotpatching")]
use crate::{change_detection::DetectChanges, HotPatchChanges};
use crate::{
    error::{ErrorContext, ErrorHandler},
    schedule::{
        executor::is_apply_deferred, ConditionWithAccess, ExecutorKind, SystemExecutor,
        SystemSchedule,
    },
    system::RunSystemError,
    world::World,
};

use super::__rust_begin_short_backtrace;

/// A variant of [`SingleThreadedExecutor`](crate::schedule::SingleThreadedExecutor) that calls
/// [`apply_deferred`](crate::system::System::apply_deferred) immediately after running each system.
#[derive(Default)]
#[deprecated(
    since = "0.17.0",
    note = "Use SingleThreadedExecutor instead. See https://github.com/bevyengine/bevy/issues/18453 for motivation."
)]
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

    fn init(&mut self, schedule: &SystemSchedule) {
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();
        self.evaluated_sets = FixedBitSet::with_capacity(set_count);
        self.completed_systems = FixedBitSet::with_capacity(sys_count);
    }

    fn run(
        &mut self,
        schedule: &mut SystemSchedule,
        world: &mut World,
        _skip_systems: Option<&FixedBitSet>,
        error_handler: ErrorHandler,
    ) {
        // If stepping is enabled, make sure we skip those systems that should
        // not be run.
        #[cfg(feature = "bevy_debug_stepping")]
        if let Some(skipped_systems) = _skip_systems {
            // mark skipped systems as completed
            self.completed_systems |= skipped_systems;
        }

        #[cfg(feature = "hotpatching")]
        let hotpatch_tick = world
            .get_resource_ref::<HotPatchChanges>()
            .map(|r| r.last_changed())
            .unwrap_or_default();

        for system_index in 0..schedule.systems.len() {
            #[cfg(feature = "trace")]
            let name = schedule.systems[system_index].system.name();
            #[cfg(feature = "trace")]
            let should_run_span = info_span!("check_conditions", name = name.as_string()).entered();

            let mut should_run = !self.completed_systems.contains(system_index);
            for set_idx in schedule.sets_with_conditions_of_systems[system_index].ones() {
                if self.evaluated_sets.contains(set_idx) {
                    continue;
                }

                // evaluate system set's conditions
                let set_conditions_met = evaluate_and_fold_conditions(
                    &mut schedule.set_conditions[set_idx],
                    world,
                    error_handler,
                );

                if !set_conditions_met {
                    self.completed_systems
                        .union_with(&schedule.systems_in_sets_with_conditions[set_idx]);
                }

                should_run &= set_conditions_met;
                self.evaluated_sets.insert(set_idx);
            }

            // evaluate system's conditions
            let system_conditions_met = evaluate_and_fold_conditions(
                &mut schedule.system_conditions[system_index],
                world,
                error_handler,
            );

            should_run &= system_conditions_met;

            let system = &mut schedule.systems[system_index].system;

            #[cfg(feature = "trace")]
            should_run_span.exit();

            #[cfg(feature = "hotpatching")]
            if hotpatch_tick.is_newer_than(system.get_last_run(), world.change_tick()) {
                system.refresh_hotpatch();
            }

            // system has either been skipped or will run
            self.completed_systems.insert(system_index);

            if !should_run {
                continue;
            }

            if is_apply_deferred(&**system) {
                continue;
            }

            let f = AssertUnwindSafe(|| {
                if let Err(RunSystemError::Failed(err)) =
                    __rust_begin_short_backtrace::run(system, world)
                {
                    error_handler(
                        err,
                        ErrorContext::System {
                            name: system.name(),
                            last_run: system.get_last_run(),
                        },
                    );
                }
            });

            #[cfg(feature = "std")]
            #[expect(clippy::print_stderr, reason = "Allowed behind `std` feature gate.")]
            {
                if let Err(payload) = std::panic::catch_unwind(f) {
                    eprintln!("Encountered a panic in system `{}`!", system.name());
                    std::panic::resume_unwind(payload);
                }
            }

            #[cfg(not(feature = "std"))]
            {
                (f)();
            }
        }

        self.evaluated_sets.clear();
        self.completed_systems.clear();
    }

    fn set_apply_final_deferred(&mut self, _: bool) {
        // do nothing. simple executor does not do a final sync
    }
}

impl SimpleExecutor {
    /// Creates a new simple executor for use in a [`Schedule`](crate::schedule::Schedule).
    /// This calls each system in order and immediately calls [`System::apply_deferred`](crate::system::System).
    pub const fn new() -> Self {
        Self {
            evaluated_sets: FixedBitSet::new(),
            completed_systems: FixedBitSet::new(),
        }
    }
}
#[deprecated(
    since = "0.17.0",
    note = "Use SingleThreadedExecutor instead. See https://github.com/bevyengine/bevy/issues/18453 for motivation."
)]
fn evaluate_and_fold_conditions(
    conditions: &mut [ConditionWithAccess],
    world: &mut World,
    error_handler: ErrorHandler,
) -> bool {
    #[cfg(feature = "hotpatching")]
    let hotpatch_tick = world
        .get_resource_ref::<HotPatchChanges>()
        .map(|r| r.last_changed())
        .unwrap_or_default();

    #[expect(
        clippy::unnecessary_fold,
        reason = "Short-circuiting here would prevent conditions from mutating their own state as needed."
    )]
    conditions
        .iter_mut()
        .map(|ConditionWithAccess { condition, .. }| {
            #[cfg(feature = "hotpatching")]
            if hotpatch_tick.is_newer_than(condition.get_last_run(), world.change_tick()) {
                condition.refresh_hotpatch();
            }
            __rust_begin_short_backtrace::readonly_run(&mut **condition, world).unwrap_or_else(
                |err| {
                    if let RunSystemError::Failed(err) = err {
                        error_handler(
                            err,
                            ErrorContext::RunCondition {
                                name: condition.name(),
                                last_run: condition.get_last_run(),
                            },
                        );
                    };
                    false
                },
            )
        })
        .fold(true, |acc, res| acc && res)
}

#[cfg(test)]
#[test]
fn skip_automatic_sync_points() {
    // Schedules automatically insert ApplyDeferred systems, but these should
    // not be executed as they only serve as markers and are not initialized
    use crate::prelude::*;
    let mut sched = Schedule::default();
    sched.set_executor_kind(ExecutorKind::Simple);
    sched.add_systems((|_: Commands| (), || ()).chain());
    let mut world = World::new();
    sched.run(&mut world);
}
