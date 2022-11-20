use bevy_tasks::{ComputeTaskPool, Scope, TaskPool};
use bevy_utils::default;
use bevy_utils::syncunsafecell::SyncUnsafeCell;
#[cfg(feature = "trace")]
use bevy_utils::tracing::{info_span, Instrument};

use async_channel::{Receiver, Sender};
use fixedbitset::FixedBitSet;

use crate::{
    archetype::ArchetypeComponentId,
    query::Access,
    schedule_v3::{is_apply_system_buffers, ExecutorKind, SystemExecutor, SystemSchedule},
    world::World,
};

/// Per-system data used by the [`MultiThreadedExecutor`].
struct SystemTaskMetadata {
    /// Indices of the systems that directly depend on the system.
    dependents: Vec<usize>,
    /// The number of dependencies the system has in total.
    dependencies_total: usize,
    /// The number of dependencies the system has that have not completed.
    dependencies_remaining: usize,
    // These values are cached because we can't read them from the system while it's running.
    /// The `ArchetypeComponentId` access of the system.
    archetype_component_access: Access<ArchetypeComponentId>,
    /// Is `true` if the system does not access `!Send` data.
    is_send: bool,
    /// Is `true` if the system is exclusive.
    is_exclusive: bool,
}

/// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
pub struct MultiThreadedExecutor {
    /// Metadata for scheduling and running system tasks.
    system_task_metadata: Vec<SystemTaskMetadata>,

    /// Sends system completion events.
    sender: Sender<usize>,
    /// Receives system completion events.
    receiver: Receiver<usize>,
    /// Scratch vector to avoid frequent allocation.
    dependents_scratch: Vec<usize>,

    /// Union of the accesses of all currently running systems.
    active_access: Access<ArchetypeComponentId>,
    /// Returns `true` if a system with non-`Send` access is running.
    local_thread_running: bool,
    /// Returns `true` if an exclusive system is running.
    exclusive_running: bool,

    /// System sets that have been skipped or had their conditions evaluated.
    completed_sets: FixedBitSet,
    /// Systems that have run or been skipped.
    completed_systems: FixedBitSet,
    /// Systems that have no remaining dependencies and are waiting to run.
    ready_systems: FixedBitSet,
    /// Used to avoid checking systems twice.
    seen_ready_systems: FixedBitSet,
    /// Systems that are currently running.
    running_systems: FixedBitSet,
    /// Systems that have run but have not had their buffers applied.
    unapplied_systems: FixedBitSet,
}

impl Default for MultiThreadedExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemExecutor for MultiThreadedExecutor {
    fn kind(&self) -> ExecutorKind {
        ExecutorKind::MultiThreaded
    }

    fn init(&mut self, schedule: &SystemSchedule) {
        // pre-allocate space
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();

        self.completed_sets = FixedBitSet::with_capacity(set_count);

        self.completed_systems = FixedBitSet::with_capacity(sys_count);
        self.ready_systems = FixedBitSet::with_capacity(sys_count);
        self.seen_ready_systems = FixedBitSet::with_capacity(sys_count);
        self.running_systems = FixedBitSet::with_capacity(sys_count);
        self.unapplied_systems = FixedBitSet::with_capacity(sys_count);

        self.dependents_scratch = Vec::with_capacity(sys_count);
        self.system_task_metadata = Vec::with_capacity(sys_count);

        for index in 0..sys_count {
            let (num_dependencies, dependents) = schedule.system_deps[index].clone();
            let system = schedule.systems[index].borrow();
            self.system_task_metadata.push(SystemTaskMetadata {
                dependents,
                dependencies_total: num_dependencies,
                dependencies_remaining: num_dependencies,
                is_send: system.is_send(),
                is_exclusive: system.is_exclusive(),
                archetype_component_access: default(),
            });
        }
    }

    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        // The start of schedule execution is the best time to do this.
        world.check_change_ticks();

        #[cfg(feature = "trace")]
        let _schedule_span = info_span!("schedule").entered();
        ComputeTaskPool::init(TaskPool::default).scope(|scope| {
            // the runner itself is a `Send` future so that it can run
            // alongside systems that claim the local thread
            let runner = async {
                // systems with zero dependencies
                for (index, system_meta) in self.system_task_metadata.iter_mut().enumerate() {
                    if system_meta.dependencies_total == 0 {
                        self.ready_systems.insert(index);
                    }
                }

                // main loop
                let world = SyncUnsafeCell::from_mut(world);
                while self.completed_systems.count_ones(..) != self.completed_systems.len() {
                    if !self.exclusive_running {
                        self.spawn_system_tasks(scope, schedule, world);
                    }

                    if self.running_systems.count_ones(..) > 0 {
                        // wait for systems to complete
                        let index = self
                            .receiver
                            .recv()
                            .await
                            .unwrap_or_else(|error| unreachable!("{}", error));

                        self.finish_system_and_signal_dependents(index);

                        while let Ok(index) = self.receiver.try_recv() {
                            self.finish_system_and_signal_dependents(index);
                        }

                        self.rebuild_active_access();
                    }
                }

                // SAFETY: all systems have completed
                let world = unsafe { &mut *world.get() };
                Self::apply_system_buffers(&mut self.unapplied_systems, schedule, world);

                debug_assert_eq!(self.ready_systems.count_ones(..), 0);
                debug_assert_eq!(self.running_systems.count_ones(..), 0);
                debug_assert_eq!(self.unapplied_systems.count_ones(..), 0);
                self.active_access.clear();
                self.completed_sets.clear();
                self.completed_systems.clear();
            };

            #[cfg(feature = "trace")]
            let runner_span = info_span!("schedule_task");
            #[cfg(feature = "trace")]
            let runner = runner.instrument(runner_span);
            scope.spawn(runner);
        });
    }
}

impl MultiThreadedExecutor {
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self {
            system_task_metadata: Vec::new(),
            sender,
            receiver,
            dependents_scratch: Vec::new(),
            active_access: default(),
            local_thread_running: false,
            exclusive_running: false,
            completed_sets: FixedBitSet::new(),
            completed_systems: FixedBitSet::new(),
            ready_systems: FixedBitSet::new(),
            seen_ready_systems: FixedBitSet::new(),
            running_systems: FixedBitSet::new(),
            unapplied_systems: FixedBitSet::new(),
        }
    }

    fn spawn_system_tasks<'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        schedule: &'scope SystemSchedule,
        world: &'scope SyncUnsafeCell<World>,
    ) {
        while let Some(system_index) = self
            .ready_systems
            .difference(&self.seen_ready_systems)
            .next()
        {
            // skip systems we've already seen during this call
            self.seen_ready_systems.insert(system_index);

            if !self.system_task_metadata[system_index].is_exclusive {
                // SAFETY: no exclusive system running
                let world = unsafe { &*world.get() };
                if !self.can_run(system_index, schedule, world) {
                    continue;
                }
                if !self.should_run(system_index, schedule, world) {
                    continue;
                }
                self.spawn_system_task(scope, system_index, schedule, world);
            } else {
                {
                    // SAFETY: no exclusive system running
                    let world = unsafe { &*world.get() };
                    if !self.can_run(system_index, schedule, world) {
                        // the `break` here emulates single-threaded runner behavior
                        // without it, exclusive systems would likely be stalled out
                        break;
                    }
                    if !self.should_run(system_index, schedule, world) {
                        continue;
                    }
                }
                // SAFETY: no system running
                let world = unsafe { &mut *world.get() };
                self.spawn_exclusive_system_task(scope, system_index, schedule, world);
                break;
            }
        }

        self.seen_ready_systems.clear();
    }

    fn spawn_system_task<'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        system_index: usize,
        schedule: &'scope SystemSchedule,
        world: &'scope World,
    ) {
        // SAFETY: system was not already running
        let system = unsafe { &mut *schedule.systems[system_index].as_ptr() };

        #[cfg(feature = "trace")]
        let task_span = info_span!("system_task", name = &*system.name());
        #[cfg(feature = "trace")]
        let system_span = info_span!("system", name = &*system.name());

        let sender = self.sender.clone();
        let task = async move {
            #[cfg(feature = "trace")]
            let system_guard = system_span.enter();
            // SAFETY: access is compatible
            unsafe { system.run_unsafe((), world) };
            #[cfg(feature = "trace")]
            drop(system_guard);
            sender
                .send(system_index)
                .await
                .unwrap_or_else(|error| unreachable!("{}", error));
        };

        #[cfg(feature = "trace")]
        let task = task.instrument(task_span);

        let system_meta = &self.system_task_metadata[system_index];
        self.active_access
            .extend(&system_meta.archetype_component_access);

        self.ready_systems.set(system_index, false);
        self.running_systems.insert(system_index);

        if system_meta.is_send {
            scope.spawn(task);
        } else {
            self.local_thread_running = true;
            scope.spawn_on_scope(task);
        }
    }

    fn spawn_exclusive_system_task<'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        system_index: usize,
        schedule: &'scope SystemSchedule,
        world: &'scope mut World,
    ) {
        // SAFETY: system was not already running
        let system = unsafe { &mut *schedule.systems[system_index].as_ptr() };

        #[cfg(feature = "trace")]
        let task_span = info_span!("system_task", name = &*system.name());
        #[cfg(feature = "trace")]
        let system_span = info_span!("system", name = &*system.name());

        let sender = self.sender.clone();

        self.ready_systems.set(system_index, false);
        self.running_systems.insert(system_index);
        self.local_thread_running = true;
        self.exclusive_running = true;

        if is_apply_system_buffers(system) {
            // TODO: avoid allocation
            let mut unapplied_systems = self.unapplied_systems.clone();
            let task = async move {
                #[cfg(feature = "trace")]
                let system_guard = system_span.enter();
                Self::apply_system_buffers(&mut unapplied_systems, schedule, world);
                #[cfg(feature = "trace")]
                drop(system_guard);
                sender
                    .send(system_index)
                    .await
                    .unwrap_or_else(|error| unreachable!("{}", error));
            };

            #[cfg(feature = "trace")]
            let task = task.instrument(task_span);
            scope.spawn_on_scope(task);
        } else {
            let task = async move {
                #[cfg(feature = "trace")]
                let system_guard = system_span.enter();
                system.run((), world);
                #[cfg(feature = "trace")]
                drop(system_guard);
                sender
                    .send(system_index)
                    .await
                    .unwrap_or_else(|error| unreachable!("{}", error));
            };

            #[cfg(feature = "trace")]
            let task = task.instrument(task_span);
            scope.spawn_on_scope(task);
        }
    }

    fn can_run(&mut self, system_index: usize, schedule: &SystemSchedule, world: &World) -> bool {
        #[cfg(feature = "trace")]
        let name = schedule.systems[system_index].borrow().name();
        #[cfg(feature = "trace")]
        let _span = info_span!("check_access", name = &*name).entered();

        let system_meta = &mut self.system_task_metadata[system_index];
        if self.local_thread_running && !system_meta.is_send {
            // only one thread can access thread-local resources
            return false;
        }

        let mut system = schedule.systems[system_index].borrow_mut();
        system.update_archetype_component_access(world);

        // TODO: avoid allocation
        system_meta.archetype_component_access = system.archetype_component_access().clone();
        let mut total_access = system.archetype_component_access().clone();

        let mut system_conditions = schedule.system_conditions[system_index].borrow_mut();
        for condition in system_conditions.iter_mut() {
            condition.update_archetype_component_access(world);
            total_access.extend(condition.archetype_component_access());
        }

        for set_idx in schedule.sets_of_systems[system_index].difference(&self.completed_sets) {
            let mut set_conditions = schedule.set_conditions[set_idx].borrow_mut();
            for condition in set_conditions.iter_mut() {
                condition.update_archetype_component_access(world);
                total_access.extend(condition.archetype_component_access());
            }
        }

        total_access.is_compatible(&self.active_access)
    }

    fn should_run(
        &mut self,
        system_index: usize,
        schedule: &SystemSchedule,
        world: &World,
    ) -> bool {
        #[cfg(feature = "trace")]
        let name = schedule.systems[system_index].borrow().name();
        #[cfg(feature = "trace")]
        let _span = info_span!("check_conditions", name = &*name).entered();

        // evaluate conditions
        let mut should_run = true;

        // evaluate set conditions in hierarchical order
        for set_idx in schedule.sets_of_systems[system_index].ones() {
            if self.completed_sets.contains(set_idx) {
                continue;
            }

            let mut set_conditions = schedule.set_conditions[set_idx].borrow_mut();

            // if any condition fails, we need to restore their change ticks
            let saved_tick = set_conditions
                .iter()
                .map(|condition| condition.get_last_change_tick())
                .min();

            let set_conditions_met = set_conditions.iter_mut().all(|condition| {
                #[cfg(feature = "trace")]
                let _condition_span = info_span!("condition", name = &*condition.name()).entered();
                // SAFETY: access is compatible
                unsafe { condition.run_unsafe((), world) }
            });

            self.completed_sets.insert(set_idx);

            if !set_conditions_met {
                // mark all members as completed
                for sys_idx in schedule.systems_of_sets[set_idx].ones() {
                    if !self.completed_systems.contains(sys_idx) {
                        self.skip_system_and_signal_dependents(sys_idx);
                    }
                }

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
            return false;
        }

        let system = schedule.systems[system_index].borrow();

        // evaluate the system's conditions
        let mut system_conditions = schedule.system_conditions[system_index].borrow_mut();
        for condition in system_conditions.iter_mut() {
            condition.set_last_change_tick(system.get_last_change_tick());
        }

        let should_run = system_conditions.iter_mut().all(|condition| {
            #[cfg(feature = "trace")]
            let _condition_span = info_span!("condition", name = &*condition.name()).entered();
            // SAFETY: access is compatible
            unsafe { condition.run_unsafe((), world) }
        });

        if !should_run {
            self.skip_system_and_signal_dependents(system_index);
            return false;
        }

        true
    }

    fn finish_system_and_signal_dependents(&mut self, system_index: usize) {
        if !self.system_task_metadata[system_index].is_send {
            self.local_thread_running = false;
        }

        if self.system_task_metadata[system_index].is_exclusive {
            self.exclusive_running = false;
        }

        self.running_systems.set(system_index, false);
        self.completed_systems.insert(system_index);
        self.unapplied_systems.insert(system_index);
        self.signal_dependents(system_index);
    }

    fn skip_system_and_signal_dependents(&mut self, system_index: usize) {
        self.ready_systems.set(system_index, false);
        self.completed_systems.insert(system_index);
        self.signal_dependents(system_index);
    }

    fn signal_dependents(&mut self, system_index: usize) {
        #[cfg(feature = "trace")]
        let _span = info_span!("signal_dependents").entered();
        self.dependents_scratch
            .extend_from_slice(&self.system_task_metadata[system_index].dependents);

        for &dep_idx in &self.dependents_scratch {
            let dependent_meta = &mut self.system_task_metadata[dep_idx];
            dependent_meta.dependencies_remaining -= 1;
            if (dependent_meta.dependencies_remaining == 0)
                && !self.completed_systems.contains(dep_idx)
            {
                self.ready_systems.insert(dep_idx);
            }
        }

        self.dependents_scratch.clear();
    }

    fn rebuild_active_access(&mut self) {
        self.active_access.clear();
        for index in self.running_systems.ones() {
            let system_meta = &self.system_task_metadata[index];
            self.active_access
                .extend(&system_meta.archetype_component_access);
        }
    }

    fn apply_system_buffers(
        unapplied_systems: &mut FixedBitSet,
        schedule: &SystemSchedule,
        world: &mut World,
    ) {
        for system_index in unapplied_systems.ones() {
            let mut system = schedule.systems[system_index].borrow_mut();
            #[cfg(feature = "trace")]
            let _apply_buffers_span = info_span!("apply_buffers", name = &*system.name()).entered();
            system.apply_buffers(world);
        }

        unapplied_systems.clear();
    }
}
