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
    schedule_v3::{
        is_apply_system_buffers, BoxedCondition, ExecutorKind, SystemExecutor, SystemSchedule,
    },
    world::World,
};

/// Per-system data used by the [`MultiThreadedExecutor`].
// Copied here because it can't be read from the system when it's running.
struct SystemTaskMeta {
    /// Indices of the systems that directly depend on the system.
    dependents: Vec<usize>,
    /// The `ArchetypeComponentId` access of the system.
    archetype_component_access: Access<ArchetypeComponentId>,
    /// Is `true` if the system does not access `!Send` data.
    is_send: bool,
    /// Is `true` if the system is exclusive.
    is_exclusive: bool,
}

/// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
pub struct MultiThreadedExecutor {
    /// Sends system completion events.
    sender: Sender<usize>,
    /// Receives system completion events.
    receiver: Receiver<usize>,
    /// Metadata for scheduling and running system tasks.
    system_task_meta: Vec<SystemTaskMeta>,
    /// The number of dependencies each system has that have not completed.
    dependencies_remaining: Vec<usize>,
    /// Union of the accesses of all currently running systems.
    active_access: Access<ArchetypeComponentId>,
    /// Returns `true` if a system with non-`Send` access is running.
    local_thread_running: bool,
    /// Returns `true` if an exclusive system is running.
    exclusive_running: bool,
    /// System sets whose conditions have been evaluated.
    evaluated_sets: FixedBitSet,
    /// Systems that have no remaining dependencies and are waiting to run.
    ready_systems: FixedBitSet,
    /// Systems that are running.
    running_systems: FixedBitSet,
    /// Systems that got skipped.
    skipped_systems: FixedBitSet,
    /// Systems whose conditions have been evaluated and were run or skipped.
    completed_systems: FixedBitSet,
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

        self.evaluated_sets = FixedBitSet::with_capacity(set_count);
        self.ready_systems = FixedBitSet::with_capacity(sys_count);
        self.running_systems = FixedBitSet::with_capacity(sys_count);
        self.completed_systems = FixedBitSet::with_capacity(sys_count);
        self.skipped_systems = FixedBitSet::with_capacity(sys_count);
        self.unapplied_systems = FixedBitSet::with_capacity(sys_count);

        self.system_task_meta = Vec::with_capacity(sys_count);
        for index in 0..sys_count {
            let system = schedule.systems[index].borrow();
            self.system_task_meta.push(SystemTaskMeta {
                dependents: schedule.system_dependents[index].clone(),
                archetype_component_access: default(),
                is_send: system.is_send(),
                is_exclusive: system.is_exclusive(),
            });
        }

        self.dependencies_remaining = Vec::with_capacity(sys_count);
    }

    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        ComputeTaskPool::init(TaskPool::default).scope(|scope| {
            // the executor itself is a `Send` future so that it can run
            // alongside systems that claim the local thread
            let executor = async {
                // reset counts
                self.dependencies_remaining.clear();
                self.dependencies_remaining
                    .extend_from_slice(&schedule.system_dependencies);

                for (system_index, dependencies) in
                    self.dependencies_remaining.iter_mut().enumerate()
                {
                    if *dependencies == 0 {
                        self.ready_systems.insert(system_index);
                    }
                }

                // using spare bitset to avoid repeated allocations
                let mut ready_systems = FixedBitSet::with_capacity(self.ready_systems.len());

                // main loop
                let world = SyncUnsafeCell::from_mut(world);
                while self.completed_systems.count_ones(..) != self.completed_systems.len() {
                    if !self.exclusive_running {
                        ready_systems.clear();
                        ready_systems.union_with(&self.ready_systems);
                        self.spawn_system_tasks(&ready_systems, scope, schedule, world);
                    }

                    if !self.running_systems.is_clear() {
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

                debug_assert!(self.ready_systems.is_clear());
                debug_assert!(self.running_systems.is_clear());
                debug_assert!(self.unapplied_systems.is_clear());
                self.active_access.clear();
                self.evaluated_sets.clear();
                self.completed_systems.clear();
            };

            #[cfg(feature = "trace")]
            let executor_span = info_span!("schedule_task");
            #[cfg(feature = "trace")]
            let executor = executor.instrument(executor_span);
            scope.spawn(executor);
        });
    }
}

impl MultiThreadedExecutor {
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self {
            sender,
            receiver,
            system_task_meta: Vec::new(),
            dependencies_remaining: Vec::new(),
            active_access: default(),
            local_thread_running: false,
            exclusive_running: false,
            evaluated_sets: FixedBitSet::new(),
            ready_systems: FixedBitSet::new(),
            running_systems: FixedBitSet::new(),
            skipped_systems: FixedBitSet::new(),
            completed_systems: FixedBitSet::new(),
            unapplied_systems: FixedBitSet::new(),
        }
    }

    fn spawn_system_tasks<'scope>(
        &mut self,
        ready_systems: &FixedBitSet,
        scope: &Scope<'_, 'scope, ()>,
        schedule: &'scope SystemSchedule,
        cell: &'scope SyncUnsafeCell<World>,
    ) {
        for system_index in ready_systems.ones() {
            // SAFETY: no exclusive system is running
            let world = unsafe { &*cell.get() };
            if !self.can_run(system_index, schedule, world) {
                // NOTE: exclusive systems with ambiguities are susceptible to
                // being significantly displaced here (compared to single-threaded order)
                // if systems after them in topological order can run
                // if that becomes an issue, `break;` if exclusive system
                continue;
            }

            // system is either going to run or be skipped
            self.ready_systems.set(system_index, false);

            if !self.should_run(system_index, schedule, world) {
                self.skip_system_and_signal_dependents(system_index);
                continue;
            }

            // system is starting
            self.running_systems.insert(system_index);

            if self.system_task_meta[system_index].is_exclusive {
                // SAFETY: `can_run` confirmed no other systems are running
                let world = unsafe { &mut *cell.get() };
                self.spawn_exclusive_system_task(scope, system_index, schedule, world);
                break;
            }

            self.spawn_system_task(scope, system_index, schedule, world);
        }
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

        let system_meta = &self.system_task_meta[system_index];
        self.active_access
            .extend(&system_meta.archetype_component_access);

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

        self.local_thread_running = true;
        self.exclusive_running = true;
    }

    fn can_run(&mut self, system_index: usize, schedule: &SystemSchedule, world: &World) -> bool {
        #[cfg(feature = "trace")]
        let name = schedule.systems[system_index].borrow().name();
        #[cfg(feature = "trace")]
        let _span = info_span!("check_access", name = &*name).entered();

        // TODO: an earlier out if archetypes did not change
        let system_meta = &mut self.system_task_meta[system_index];

        if system_meta.is_exclusive && !self.running_systems.is_clear() {
            return false;
        }

        if !system_meta.is_send && self.local_thread_running {
            return false;
        }

        for set_idx in schedule.sets_of_systems[system_index].difference(&self.evaluated_sets) {
            for condition in schedule.set_conditions[set_idx].borrow_mut().iter_mut() {
                condition.update_archetype_component_access(world);
                if !condition
                    .archetype_component_access()
                    .is_compatible(&self.active_access)
                {
                    return false;
                }
            }
        }

        for condition in schedule.system_conditions[system_index]
            .borrow_mut()
            .iter_mut()
        {
            condition.update_archetype_component_access(world);
            if !condition
                .archetype_component_access()
                .is_compatible(&self.active_access)
            {
                return false;
            }
        }

        if !self.skipped_systems.contains(system_index) {
            let mut system = schedule.systems[system_index].borrow_mut();
            system.update_archetype_component_access(world);
            if !system
                .archetype_component_access()
                .is_compatible(&self.active_access)
            {
                return false;
            }

            // TODO: avoid allocation by keeping count of readers
            system_meta.archetype_component_access = system.archetype_component_access().clone();
        }

        true
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

        let mut should_run = !self.completed_systems.contains(system_index);
        for set_idx in schedule.sets_of_systems[system_index].ones() {
            if self.evaluated_sets.contains(set_idx) {
                continue;
            }

            // evaluate system set's conditions
            let set_conditions_met = evaluate_and_fold_conditions(
                schedule.set_conditions[set_idx].borrow_mut().as_mut(),
                world,
            );

            if !set_conditions_met {
                self.skipped_systems
                    .union_with(&schedule.systems_in_sets[set_idx]);
            }

            should_run &= set_conditions_met;
            self.evaluated_sets.insert(set_idx);
        }

        // evaluate system's conditions
        let system_conditions_met = evaluate_and_fold_conditions(
            schedule.system_conditions[system_index]
                .borrow_mut()
                .as_mut(),
            world,
        );

        if !system_conditions_met {
            self.skipped_systems.insert(system_index);
        }

        should_run &= system_conditions_met;

        should_run
    }

    fn finish_system_and_signal_dependents(&mut self, system_index: usize) {
        if !self.system_task_meta[system_index].is_send {
            self.local_thread_running = false;
        }

        if self.system_task_meta[system_index].is_exclusive {
            self.exclusive_running = false;
        }

        self.running_systems.set(system_index, false);
        self.completed_systems.insert(system_index);
        self.unapplied_systems.insert(system_index);
        self.signal_dependents(system_index);
    }

    fn skip_system_and_signal_dependents(&mut self, system_index: usize) {
        self.completed_systems.insert(system_index);
        self.signal_dependents(system_index);
    }

    fn signal_dependents(&mut self, system_index: usize) {
        #[cfg(feature = "trace")]
        let _span = info_span!("signal_dependents").entered();
        for &dep_idx in &self.system_task_meta[system_index].dependents {
            let dependencies = &mut self.dependencies_remaining[dep_idx];
            *dependencies -= 1;
            if *dependencies == 0 && !self.completed_systems.contains(dep_idx) {
                self.ready_systems.insert(dep_idx);
            }
        }
    }

    fn rebuild_active_access(&mut self) {
        self.active_access.clear();
        for index in self.running_systems.ones() {
            let system_meta = &self.system_task_meta[index];
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

fn evaluate_and_fold_conditions(conditions: &mut [BoxedCondition], world: &World) -> bool {
    // not short-circuiting is intentional
    #[allow(clippy::unnecessary_fold)]
    conditions
        .iter_mut()
        .map(|condition| {
            #[cfg(feature = "trace")]
            let _condition_span = info_span!("condition", name = &*condition.name()).entered();
            // SAFETY: caller ensures system access is compatible
            unsafe { condition.run_unsafe((), world) }
        })
        .fold(true, |acc, res| acc && res)
}
