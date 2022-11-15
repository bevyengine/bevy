#[cfg(feature = "trace")]
use bevy_utils::tracing::{info_span, Instrument};
use fixedbitset::FixedBitSet;

use crate::{
    schedule_v3::{is_apply_system_buffers, SystemExecutor, SystemSchedule},
    world::World,
};

/// Runs the schedule using a single thread.
#[derive(Default)]
pub struct SingleThreadedExecutor {
    /// System sets whose conditions have either been evaluated or skipped.
    completed_sets: FixedBitSet,
    /// Systems that have run or been skipped.
    completed_systems: FixedBitSet,
    /// Systems that have run but have not had their buffers applied.
    unapplied_systems: FixedBitSet,
}

impl SystemExecutor for SingleThreadedExecutor {
    fn init(&mut self, schedule: &SystemSchedule) {
        // pre-allocate space
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();
        self.completed_sets = FixedBitSet::with_capacity(set_count);
        self.completed_systems = FixedBitSet::with_capacity(sys_count);
        self.unapplied_systems = FixedBitSet::with_capacity(sys_count);
    }

    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        // The start of schedule execution is the best time to do this.
        world.check_change_ticks();

        #[cfg(feature = "trace")]
        let _schedule_span = info_span!("schedule").entered();
        for sys_idx in 0..schedule.systems.len() {
            if self.completed_systems.contains(sys_idx) {
                continue;
            }

            #[cfg(feature = "trace")]
            let name = schedule.systems[sys_idx].get_mut().name();
            #[cfg(feature = "trace")]
            let should_run_span = info_span!("check_conditions", name = &*name).entered();

            // evaluate conditions
            let mut should_run = true;

            // evaluate set conditions in hierarchical order
            for set_idx in schedule.sets_of_systems[sys_idx].ones() {
                if self.completed_sets.contains(set_idx) {
                    continue;
                }

                let set_conditions = schedule.set_conditions[set_idx].get_mut();

                // if any condition fails, we need to restore their change ticks
                let saved_tick = set_conditions
                    .iter()
                    .map(|condition| condition.get_last_change_tick())
                    .min();

                let set_conditions_met = set_conditions.iter_mut().all(|condition| {
                    #[cfg(feature = "trace")]
                    let _condition_span =
                        info_span!("condition", name = &*condition.name()).entered();
                    condition.run((), world)
                });

                self.completed_sets.insert(set_idx);

                if !set_conditions_met {
                    // mark all members as completed
                    self.completed_systems
                        .union_with(&schedule.systems_of_sets[set_idx]);
                    self.completed_sets
                        .union_with(&schedule.sets_of_sets[set_idx]);

                    // restore condition change ticks
                    for condition in set_conditions.iter_mut() {
                        condition.set_last_change_tick(saved_tick.unwrap());
                    }
                }

                should_run &= set_conditions_met;
            }

            if !should_run {
                continue;
            }

            let system = schedule.systems[sys_idx].get_mut();

            // evaluate the system's conditions
            let system_conditions = schedule.system_conditions[sys_idx].get_mut();
            for condition in system_conditions.iter_mut() {
                condition.set_last_change_tick(system.get_last_change_tick());
            }

            let should_run = system_conditions.iter_mut().all(|condition| {
                #[cfg(feature = "trace")]
                let _condition_span = info_span!("condition", name = &*condition.name()).entered();
                condition.run((), world)
            });

            #[cfg(feature = "trace")]
            should_run_span.exit();

            // mark system as completed regardless
            self.completed_systems.insert(sys_idx);

            if !should_run {
                continue;
            }

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
                self.unapplied_systems.set(sys_idx, true);
            }
        }

        self.apply_system_buffers(schedule, world);
    }
}

impl SingleThreadedExecutor {
    pub const fn new() -> Self {
        Self {
            completed_sets: FixedBitSet::new(),
            completed_systems: FixedBitSet::new(),
            unapplied_systems: FixedBitSet::new(),
        }
    }

    fn apply_system_buffers(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        for sys_idx in self.unapplied_systems.ones() {
            let system = schedule.systems[sys_idx].get_mut();
            #[cfg(feature = "trace")]
            let _apply_buffers_span = info_span!("apply_buffers", name = &*system.name()).entered();
            system.apply_buffers(world);
        }

        self.unapplied_systems.clear();
    }
}
