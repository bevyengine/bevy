use std::{
    any::Any,
    sync::{Arc, Mutex},
};

use bevy_tasks::{ComputeTaskPool, Scope, TaskPool, ThreadExecutor};
use bevy_utils::default;
use bevy_utils::syncunsafecell::SyncUnsafeCell;
#[cfg(feature = "trace")]
use bevy_utils::tracing::{info_span, Instrument, Span};
use std::panic::AssertUnwindSafe;

use async_channel::{Receiver, Sender};
use fixedbitset::FixedBitSet;

use crate::{
    archetype::ArchetypeComponentId,
    prelude::Resource,
    query::Access,
    schedule::{is_apply_deferred, BoxedCondition, ExecutorKind, SystemExecutor, SystemSchedule},
    system::BoxedSystem,
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};

use crate as bevy_ecs;

/// A funky borrow split of [`SystemSchedule`] required by the [`MultiThreadedExecutor`].
struct SyncUnsafeSchedule<'a> {
    systems: &'a [SyncUnsafeCell<BoxedSystem>],
    conditions: Conditions<'a>,
}

struct Conditions<'a> {
    system_conditions: &'a mut [Vec<BoxedCondition>],
    set_conditions: &'a mut [Vec<BoxedCondition>],
    sets_with_conditions_of_systems: &'a [FixedBitSet],
    systems_in_sets_with_conditions: &'a [FixedBitSet],
}

impl SyncUnsafeSchedule<'_> {
    fn new(schedule: &mut SystemSchedule) -> SyncUnsafeSchedule<'_> {
        SyncUnsafeSchedule {
            systems: SyncUnsafeCell::from_mut(schedule.systems.as_mut_slice()).as_slice_of_cells(),
            conditions: Conditions {
                system_conditions: &mut schedule.system_conditions,
                set_conditions: &mut schedule.set_conditions,
                sets_with_conditions_of_systems: &schedule.sets_with_conditions_of_systems,
                systems_in_sets_with_conditions: &schedule.systems_in_sets_with_conditions,
            },
        }
    }
}

/// Per-system data used by the [`MultiThreadedExecutor`].
// Copied here because it can't be read from the system when it's running.
struct SystemTaskMetadata {
    /// The [`ArchetypeComponentId`] access of the system.
    archetype_component_access: Access<ArchetypeComponentId>,
    /// Indices of the systems that directly depend on the system.
    dependents: Vec<usize>,
    /// Is `true` if the system does not access `!Send` data.
    is_send: bool,
    /// Is `true` if the system is exclusive.
    is_exclusive: bool,
    /// Cached tracing span for system task
    #[cfg(feature = "trace")]
    system_task_span: Span,
}

/// The result of running a system that is sent across a channel.
struct SystemResult {
    system_index: usize,
    success: bool,
}

/// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
pub struct MultiThreadedExecutor {
    /// Sends system completion events.
    sender: Sender<SystemResult>,
    /// Receives system completion events.
    receiver: Receiver<SystemResult>,
    /// Metadata for scheduling and running system tasks.
    system_task_metadata: Vec<SystemTaskMetadata>,
    /// Union of the accesses of all currently running systems.
    active_access: Access<ArchetypeComponentId>,
    /// Returns `true` if a system with non-`Send` access is running.
    local_thread_running: bool,
    /// Returns `true` if an exclusive system is running.
    exclusive_running: bool,
    /// The number of systems expected to run.
    num_systems: usize,
    /// The number of systems that are running.
    num_running_systems: usize,
    /// The number of systems that have completed.
    num_completed_systems: usize,
    /// The number of dependencies each system has that have not completed.
    num_dependencies_remaining: Vec<usize>,
    /// System sets whose conditions have been evaluated.
    evaluated_sets: FixedBitSet,
    /// Systems that have no remaining dependencies and are waiting to run.
    ready_systems: FixedBitSet,
    /// copy of `ready_systems`
    ready_systems_copy: FixedBitSet,
    /// Systems that are running.
    running_systems: FixedBitSet,
    /// Systems that got skipped.
    skipped_systems: FixedBitSet,
    /// Systems whose conditions have been evaluated and were run or skipped.
    completed_systems: FixedBitSet,
    /// Systems that have run but have not had their buffers applied.
    unapplied_systems: FixedBitSet,
    /// Setting when true applies deferred system buffers after all systems have run
    apply_final_deferred: bool,
    /// When set, tells the executor that a thread has panicked.
    panic_payload: Arc<Mutex<Option<Box<dyn Any + Send>>>>,
    /// When set, stops the executor from running any more systems.
    stop_spawning: bool,
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

    fn set_apply_final_deferred(&mut self, value: bool) {
        self.apply_final_deferred = value;
    }

    fn init(&mut self, schedule: &SystemSchedule) {
        // pre-allocate space
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();

        let (tx, rx) = async_channel::bounded(sys_count.max(1));

        self.sender = tx;
        self.receiver = rx;
        self.evaluated_sets = FixedBitSet::with_capacity(set_count);
        self.ready_systems = FixedBitSet::with_capacity(sys_count);
        self.ready_systems_copy = FixedBitSet::with_capacity(sys_count);
        self.running_systems = FixedBitSet::with_capacity(sys_count);
        self.completed_systems = FixedBitSet::with_capacity(sys_count);
        self.skipped_systems = FixedBitSet::with_capacity(sys_count);
        self.unapplied_systems = FixedBitSet::with_capacity(sys_count);

        self.system_task_metadata = Vec::with_capacity(sys_count);
        for index in 0..sys_count {
            self.system_task_metadata.push(SystemTaskMetadata {
                archetype_component_access: default(),
                dependents: schedule.system_dependents[index].clone(),
                is_send: schedule.systems[index].is_send(),
                is_exclusive: schedule.systems[index].is_exclusive(),
                #[cfg(feature = "trace")]
                system_task_span: info_span!(
                    "system_task",
                    name = &*schedule.systems[index].name()
                ),
            });
        }

        self.num_dependencies_remaining = Vec::with_capacity(sys_count);
    }

    fn run(&mut self, schedule: &mut SystemSchedule, world: &mut World) {
        // reset counts
        self.num_systems = schedule.systems.len();
        if self.num_systems == 0 {
            return;
        }
        self.num_running_systems = 0;
        self.num_completed_systems = 0;
        self.num_dependencies_remaining.clear();
        self.num_dependencies_remaining
            .extend_from_slice(&schedule.system_dependencies);

        for (system_index, dependencies) in self.num_dependencies_remaining.iter_mut().enumerate() {
            if *dependencies == 0 {
                self.ready_systems.insert(system_index);
            }
        }

        let thread_executor = world
            .get_resource::<MainThreadExecutor>()
            .map(|e| e.0.clone());
        let thread_executor = thread_executor.as_deref();

        let SyncUnsafeSchedule {
            systems,
            mut conditions,
        } = SyncUnsafeSchedule::new(schedule);

        ComputeTaskPool::init(TaskPool::default).scope_with_executor(
            false,
            thread_executor,
            |scope| {
                // the executor itself is a `Send` future so that it can run
                // alongside systems that claim the local thread
                let executor = async {
                    let world_cell = world.as_unsafe_world_cell();
                    while self.num_completed_systems < self.num_systems {
                        // SAFETY:
                        // - self.ready_systems does not contain running systems.
                        // - `world_cell` has mutable access to the entire world.
                        unsafe {
                            self.spawn_system_tasks(scope, systems, &mut conditions, world_cell);
                        }

                        if self.num_running_systems > 0 {
                            // wait for systems to complete
                            if let Ok(result) = self.receiver.recv().await {
                                self.finish_system_and_handle_dependents(result);
                            } else {
                                panic!("Channel closed unexpectedly!");
                            }

                            while let Ok(result) = self.receiver.try_recv() {
                                self.finish_system_and_handle_dependents(result);
                            }

                            self.rebuild_active_access();
                        }
                    }
                };

                #[cfg(feature = "trace")]
                let executor_span = info_span!("multithreaded executor");
                #[cfg(feature = "trace")]
                let executor = executor.instrument(executor_span);
                scope.spawn(executor);
            },
        );

        if self.apply_final_deferred {
            // Do one final apply buffers after all systems have completed
            // Commands should be applied while on the scope's thread, not the executor's thread
            let res = apply_deferred(&self.unapplied_systems, systems, world);
            if let Err(payload) = res {
                let mut panic_payload = self.panic_payload.lock().unwrap();
                *panic_payload = Some(payload);
            }
            self.unapplied_systems.clear();
            debug_assert!(self.unapplied_systems.is_clear());
        }

        // check to see if there was a panic
        let mut payload = self.panic_payload.lock().unwrap();
        if let Some(payload) = payload.take() {
            std::panic::resume_unwind(payload);
        }

        debug_assert!(self.ready_systems.is_clear());
        debug_assert!(self.running_systems.is_clear());
        self.active_access.clear();
        self.evaluated_sets.clear();
        self.skipped_systems.clear();
        self.completed_systems.clear();
    }
}

impl MultiThreadedExecutor {
    /// Creates a new multi-threaded executor for use with a [`Schedule`].
    ///
    /// [`Schedule`]: crate::schedule::Schedule
    pub fn new() -> Self {
        let (sender, receiver) = async_channel::unbounded();
        Self {
            sender,
            receiver,
            system_task_metadata: Vec::new(),
            num_systems: 0,
            num_running_systems: 0,
            num_completed_systems: 0,
            num_dependencies_remaining: Vec::new(),
            active_access: default(),
            local_thread_running: false,
            exclusive_running: false,
            evaluated_sets: FixedBitSet::new(),
            ready_systems: FixedBitSet::new(),
            ready_systems_copy: FixedBitSet::new(),
            running_systems: FixedBitSet::new(),
            skipped_systems: FixedBitSet::new(),
            completed_systems: FixedBitSet::new(),
            unapplied_systems: FixedBitSet::new(),
            apply_final_deferred: true,
            panic_payload: Arc::new(Mutex::new(None)),
            stop_spawning: false,
        }
    }

    /// # Safety
    /// - Caller must ensure that `self.ready_systems` does not contain any systems that
    ///   have been mutably borrowed (such as the systems currently running).
    /// - `world_cell` must have permission to access all world data (not counting
    ///   any world data that is claimed by systems currently running on this executor).
    unsafe fn spawn_system_tasks<'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        systems: &'scope [SyncUnsafeCell<BoxedSystem>],
        conditions: &mut Conditions,
        world_cell: UnsafeWorldCell<'scope>,
    ) {
        if self.exclusive_running {
            return;
        }

        // can't borrow since loop mutably borrows `self`
        let mut ready_systems = std::mem::take(&mut self.ready_systems_copy);
        ready_systems.clear();
        ready_systems.union_with(&self.ready_systems);

        for system_index in ready_systems.ones() {
            assert!(!self.running_systems.contains(system_index));
            // SAFETY: Caller assured that these systems are not running.
            // Therefore, no other reference to this system exists and there is no aliasing.
            let system = unsafe { &mut *systems[system_index].get() };

            if !self.can_run(system_index, system, conditions, world_cell) {
                // NOTE: exclusive systems with ambiguities are susceptible to
                // being significantly displaced here (compared to single-threaded order)
                // if systems after them in topological order can run
                // if that becomes an issue, `break;` if exclusive system
                continue;
            }

            self.ready_systems.set(system_index, false);

            // SAFETY: `can_run` returned true, which means that:
            // - It must have called `update_archetype_component_access` for each run condition.
            // - There can be no systems running whose accesses would conflict with any conditions.
            if !self.should_run(system_index, system, conditions, world_cell) {
                self.skip_system_and_signal_dependents(system_index);
                continue;
            }

            self.running_systems.insert(system_index);
            self.num_running_systems += 1;

            if self.system_task_metadata[system_index].is_exclusive {
                // SAFETY: `can_run` returned true for this system, which means
                // that no other systems currently have access to the world.
                let world = unsafe { world_cell.world_mut() };
                // SAFETY: `can_run` returned true for this system,
                // which means no systems are currently borrowed.
                unsafe {
                    self.spawn_exclusive_system_task(scope, system_index, systems, world);
                }
                break;
            }

            // SAFETY:
            // - No other reference to this system exists.
            // - `can_run` has been called, which calls `update_archetype_component_access` with this system.
            // - `can_run` returned true, so no systems with conflicting world access are running.
            unsafe {
                self.spawn_system_task(scope, system_index, systems, world_cell);
            }
        }

        // give back
        self.ready_systems_copy = ready_systems;
    }

    fn can_run(
        &mut self,
        system_index: usize,
        system: &mut BoxedSystem,
        conditions: &mut Conditions,
        world: UnsafeWorldCell,
    ) -> bool {
        let system_meta = &self.system_task_metadata[system_index];
        if system_meta.is_exclusive && self.num_running_systems > 0 {
            return false;
        }

        if !system_meta.is_send && self.local_thread_running {
            return false;
        }

        // TODO: an earlier out if world's archetypes did not change
        for set_idx in conditions.sets_with_conditions_of_systems[system_index]
            .difference(&self.evaluated_sets)
        {
            for condition in &mut conditions.set_conditions[set_idx] {
                condition.update_archetype_component_access(world);
                if !condition
                    .archetype_component_access()
                    .is_compatible(&self.active_access)
                {
                    return false;
                }
            }
        }

        for condition in &mut conditions.system_conditions[system_index] {
            condition.update_archetype_component_access(world);
            if !condition
                .archetype_component_access()
                .is_compatible(&self.active_access)
            {
                return false;
            }
        }

        if !self.skipped_systems.contains(system_index) {
            system.update_archetype_component_access(world);
            if !system
                .archetype_component_access()
                .is_compatible(&self.active_access)
            {
                return false;
            }

            // PERF: use an optimized clear() + extend() operation
            let meta_access =
                &mut self.system_task_metadata[system_index].archetype_component_access;
            meta_access.clear();
            meta_access.extend(system.archetype_component_access());
        }

        true
    }

    /// # Safety
    /// * `world` must have permission to read any world data required by
    ///   the system's conditions: this includes conditions for the system
    ///   itself, and conditions for any of the system's sets.
    /// * `update_archetype_component` must have been called with `world`
    ///   for each run condition in `conditions`.
    unsafe fn should_run(
        &mut self,
        system_index: usize,
        _system: &BoxedSystem,
        conditions: &mut Conditions,
        world: UnsafeWorldCell,
    ) -> bool {
        let mut should_run = !self.skipped_systems.contains(system_index);
        for set_idx in conditions.sets_with_conditions_of_systems[system_index].ones() {
            if self.evaluated_sets.contains(set_idx) {
                continue;
            }

            // Evaluate the system set's conditions.
            // SAFETY:
            // - The caller ensures that `world` has permission to read any data
            //   required by the conditions.
            // - `update_archetype_component_access` has been called for each run condition.
            let set_conditions_met =
                evaluate_and_fold_conditions(&mut conditions.set_conditions[set_idx], world);

            if !set_conditions_met {
                self.skipped_systems
                    .union_with(&conditions.systems_in_sets_with_conditions[set_idx]);
            }

            should_run &= set_conditions_met;
            self.evaluated_sets.insert(set_idx);
        }

        // Evaluate the system's conditions.
        // SAFETY:
        // - The caller ensures that `world` has permission to read any data
        //   required by the conditions.
        // - `update_archetype_component_access` has been called for each run condition.
        let system_conditions_met =
            evaluate_and_fold_conditions(&mut conditions.system_conditions[system_index], world);

        if !system_conditions_met {
            self.skipped_systems.insert(system_index);
        }

        should_run &= system_conditions_met;

        should_run
    }

    /// # Safety
    /// - Caller must not alias systems that are running.
    /// - `world` must have permission to access the world data
    ///   used by the specified system.
    /// - `update_archetype_component_access` must have been called with `world`
    ///   on the system associated with `system_index`.
    unsafe fn spawn_system_task<'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        system_index: usize,
        systems: &'scope [SyncUnsafeCell<BoxedSystem>],
        world: UnsafeWorldCell<'scope>,
    ) {
        // SAFETY: this system is not running, no other reference exists
        let system = unsafe { &mut *systems[system_index].get() };
        let sender = self.sender.clone();
        let panic_payload = self.panic_payload.clone();
        let task = async move {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                // SAFETY:
                // - The caller ensures that we have permission to
                // access the world data used by the system.
                // - `update_archetype_component_access` has been called.
                unsafe { system.run_unsafe((), world) };
            }));
            // tell the executor that the system finished
            sender
                .try_send(SystemResult {
                    system_index,
                    success: res.is_ok(),
                })
                .unwrap_or_else(|error| unreachable!("{}", error));
            if let Err(payload) = res {
                eprintln!("Encountered a panic in system `{}`!", &*system.name());
                // set the payload to propagate the error
                {
                    let mut panic_payload = panic_payload.lock().unwrap();
                    *panic_payload = Some(payload);
                }
            }
        };

        #[cfg(feature = "trace")]
        let task = task.instrument(
            self.system_task_metadata[system_index]
                .system_task_span
                .clone(),
        );

        let system_meta = &self.system_task_metadata[system_index];
        self.active_access
            .extend(&system_meta.archetype_component_access);

        if system_meta.is_send {
            scope.spawn(task);
        } else {
            self.local_thread_running = true;
            scope.spawn_on_external(task);
        }
    }

    /// # Safety
    /// Caller must ensure no systems are currently borrowed.
    unsafe fn spawn_exclusive_system_task<'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        system_index: usize,
        systems: &'scope [SyncUnsafeCell<BoxedSystem>],
        world: &'scope mut World,
    ) {
        // SAFETY: this system is not running, no other reference exists
        let system = unsafe { &mut *systems[system_index].get() };

        let sender = self.sender.clone();
        let panic_payload = self.panic_payload.clone();
        if is_apply_deferred(system) {
            // TODO: avoid allocation
            let unapplied_systems = self.unapplied_systems.clone();
            self.unapplied_systems.clear();
            let task = async move {
                let res = apply_deferred(&unapplied_systems, systems, world);
                // tell the executor that the system finished
                sender
                    .try_send(SystemResult {
                        system_index,
                        success: res.is_ok(),
                    })
                    .unwrap_or_else(|error| unreachable!("{}", error));
                if let Err(payload) = res {
                    // set the payload to propagate the error
                    let mut panic_payload = panic_payload.lock().unwrap();
                    *panic_payload = Some(payload);
                }
            };

            #[cfg(feature = "trace")]
            let task = task.instrument(
                self.system_task_metadata[system_index]
                    .system_task_span
                    .clone(),
            );
            scope.spawn_on_scope(task);
        } else {
            let task = async move {
                let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    system.run((), world);
                }));
                // tell the executor that the system finished
                sender
                    .try_send(SystemResult {
                        system_index,
                        success: res.is_ok(),
                    })
                    .unwrap_or_else(|error| unreachable!("{}", error));
                if let Err(payload) = res {
                    eprintln!(
                        "Encountered a panic in exclusive system `{}`!",
                        &*system.name()
                    );
                    // set the payload to propagate the error
                    let mut panic_payload = panic_payload.lock().unwrap();
                    *panic_payload = Some(payload);
                }
            };

            #[cfg(feature = "trace")]
            let task = task.instrument(
                self.system_task_metadata[system_index]
                    .system_task_span
                    .clone(),
            );
            scope.spawn_on_scope(task);
        }

        self.exclusive_running = true;
        self.local_thread_running = true;
    }

    fn finish_system_and_handle_dependents(&mut self, result: SystemResult) {
        let SystemResult {
            system_index,
            success,
        } = result;

        if self.system_task_metadata[system_index].is_exclusive {
            self.exclusive_running = false;
        }

        if !self.system_task_metadata[system_index].is_send {
            self.local_thread_running = false;
        }

        debug_assert!(self.num_running_systems >= 1);
        self.num_running_systems -= 1;
        self.num_completed_systems += 1;
        self.running_systems.set(system_index, false);
        self.completed_systems.insert(system_index);
        self.unapplied_systems.insert(system_index);

        self.signal_dependents(system_index);

        if !success {
            self.stop_spawning_systems();
        }
    }

    fn skip_system_and_signal_dependents(&mut self, system_index: usize) {
        self.num_completed_systems += 1;
        self.completed_systems.insert(system_index);
        self.signal_dependents(system_index);
    }

    fn signal_dependents(&mut self, system_index: usize) {
        for &dep_idx in &self.system_task_metadata[system_index].dependents {
            let remaining = &mut self.num_dependencies_remaining[dep_idx];
            debug_assert!(*remaining >= 1);
            *remaining -= 1;
            if *remaining == 0 && !self.completed_systems.contains(dep_idx) {
                self.ready_systems.insert(dep_idx);
            }
        }
    }

    fn stop_spawning_systems(&mut self) {
        if !self.stop_spawning {
            self.num_systems = self.num_completed_systems + self.num_running_systems;
            self.stop_spawning = true;
        }
    }

    fn rebuild_active_access(&mut self) {
        self.active_access.clear();
        for index in self.running_systems.ones() {
            let system_meta = &self.system_task_metadata[index];
            self.active_access
                .extend(&system_meta.archetype_component_access);
        }
    }
}

fn apply_deferred(
    unapplied_systems: &FixedBitSet,
    systems: &[SyncUnsafeCell<BoxedSystem>],
    world: &mut World,
) -> Result<(), Box<dyn std::any::Any + Send>> {
    for system_index in unapplied_systems.ones() {
        // SAFETY: none of these systems are running, no other references exist
        let system = unsafe { &mut *systems[system_index].get() };
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
            system.apply_deferred(world);
        }));
        if let Err(payload) = res {
            eprintln!(
                "Encountered a panic when applying buffers for system `{}`!",
                &*system.name()
            );
            return Err(payload);
        }
    }
    Ok(())
}

/// # Safety
/// - `world` must have permission to read any world data
///   required by `conditions`.
/// - `update_archetype_component_access` must have been called
///   with `world` for each condition in `conditions`.
unsafe fn evaluate_and_fold_conditions(
    conditions: &mut [BoxedCondition],
    world: UnsafeWorldCell,
) -> bool {
    // not short-circuiting is intentional
    #[allow(clippy::unnecessary_fold)]
    conditions
        .iter_mut()
        .map(|condition| {
            // SAFETY: The caller ensures that `world` has permission to
            // access any data required by the condition.
            unsafe { condition.run_unsafe((), world) }
        })
        .fold(true, |acc, res| acc && res)
}

/// New-typed [`ThreadExecutor`] [`Resource`] that is used to run systems on the main thread
#[derive(Resource, Clone)]
pub struct MainThreadExecutor(pub Arc<ThreadExecutor<'static>>);

impl Default for MainThreadExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl MainThreadExecutor {
    /// Creates a new executor that can be used to run systems on the main thread.
    pub fn new() -> Self {
        MainThreadExecutor(TaskPool::get_thread_executor())
    }
}
