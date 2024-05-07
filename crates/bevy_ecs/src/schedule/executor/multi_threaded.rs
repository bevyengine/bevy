use std::{
    any::Any,
    sync::{Arc, Mutex, MutexGuard},
};

use bevy_tasks::{ComputeTaskPool, Scope, TaskPool, ThreadExecutor};
use bevy_utils::default;
use bevy_utils::syncunsafecell::SyncUnsafeCell;
#[cfg(feature = "trace")]
use bevy_utils::tracing::{info_span, Span};
use std::panic::AssertUnwindSafe;

use concurrent_queue::ConcurrentQueue;
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

use super::__rust_begin_short_backtrace;

/// Borrowed data used by the [`MultiThreadedExecutor`].
struct Environment<'env, 'sys> {
    executor: &'env MultiThreadedExecutor,
    systems: &'sys [SyncUnsafeCell<BoxedSystem>],
    conditions: SyncUnsafeCell<Conditions<'sys>>,
    world_cell: UnsafeWorldCell<'env>,
}

struct Conditions<'a> {
    system_conditions: &'a mut [Vec<BoxedCondition>],
    set_conditions: &'a mut [Vec<BoxedCondition>],
    sets_with_conditions_of_systems: &'a [FixedBitSet],
    systems_in_sets_with_conditions: &'a [FixedBitSet],
}

impl<'env, 'sys> Environment<'env, 'sys> {
    fn new(
        executor: &'env MultiThreadedExecutor,
        schedule: &'sys mut SystemSchedule,
        world: &'env mut World,
    ) -> Self {
        Environment {
            executor,
            systems: SyncUnsafeCell::from_mut(schedule.systems.as_mut_slice()).as_slice_of_cells(),
            conditions: SyncUnsafeCell::new(Conditions {
                system_conditions: &mut schedule.system_conditions,
                set_conditions: &mut schedule.set_conditions,
                sets_with_conditions_of_systems: &schedule.sets_with_conditions_of_systems,
                systems_in_sets_with_conditions: &schedule.systems_in_sets_with_conditions,
            }),
            world_cell: world.as_unsafe_world_cell(),
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
}

/// The result of running a system that is sent across a channel.
struct SystemResult {
    system_index: usize,
}

/// Runs the schedule using a thread pool. Non-conflicting systems can run in parallel.
pub struct MultiThreadedExecutor {
    /// The running state, protected by a mutex so that a reference to the executor can be shared across tasks.
    state: Mutex<ExecutorState>,
    /// Queue of system completion events.
    system_completion: ConcurrentQueue<SystemResult>,
    /// Setting when true applies deferred system buffers after all systems have run
    apply_final_deferred: bool,
    /// When set, tells the executor that a thread has panicked.
    panic_payload: Mutex<Option<Box<dyn Any + Send>>>,
    starting_systems: FixedBitSet,
    /// Cached tracing span
    #[cfg(feature = "trace")]
    executor_span: Span,
}

/// The state of the executor while running.
pub struct ExecutorState {
    /// Metadata for scheduling and running system tasks.
    system_task_metadata: Vec<SystemTaskMetadata>,
    /// Union of the accesses of all currently running systems.
    active_access: Access<ArchetypeComponentId>,
    /// Returns `true` if a system with non-`Send` access is running.
    local_thread_running: bool,
    /// Returns `true` if an exclusive system is running.
    exclusive_running: bool,
    /// The number of systems that are running.
    num_running_systems: usize,
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
}

/// References to data required by the executor.
/// This is copied to each system task so that can invoke the executor when they complete.
// These all need to outlive 'scope in order to be sent to new tasks,
// and keeping them all in a struct means we can use lifetime elision.
#[derive(Copy, Clone)]
struct Context<'scope, 'env, 'sys> {
    environment: &'env Environment<'env, 'sys>,
    scope: &'scope Scope<'scope, 'env, ()>,
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
        let state = self.state.get_mut().unwrap();
        // pre-allocate space
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();

        self.system_completion = ConcurrentQueue::bounded(sys_count.max(1));
        self.starting_systems = FixedBitSet::with_capacity(sys_count);
        state.evaluated_sets = FixedBitSet::with_capacity(set_count);
        state.ready_systems = FixedBitSet::with_capacity(sys_count);
        state.ready_systems_copy = FixedBitSet::with_capacity(sys_count);
        state.running_systems = FixedBitSet::with_capacity(sys_count);
        state.completed_systems = FixedBitSet::with_capacity(sys_count);
        state.skipped_systems = FixedBitSet::with_capacity(sys_count);
        state.unapplied_systems = FixedBitSet::with_capacity(sys_count);

        state.system_task_metadata = Vec::with_capacity(sys_count);
        for index in 0..sys_count {
            state.system_task_metadata.push(SystemTaskMetadata {
                archetype_component_access: default(),
                dependents: schedule.system_dependents[index].clone(),
                is_send: schedule.systems[index].is_send(),
                is_exclusive: schedule.systems[index].is_exclusive(),
            });
            if schedule.system_dependencies[index] == 0 {
                self.starting_systems.insert(index);
            }
        }

        state.num_dependencies_remaining = Vec::with_capacity(sys_count);
    }

    fn run(
        &mut self,
        schedule: &mut SystemSchedule,
        world: &mut World,
        _skip_systems: Option<&FixedBitSet>,
    ) {
        let state = self.state.get_mut().unwrap();
        // reset counts
        if schedule.systems.is_empty() {
            return;
        }
        state.num_running_systems = 0;
        state
            .num_dependencies_remaining
            .clone_from(&schedule.system_dependencies);
        state.ready_systems.clone_from(&self.starting_systems);

        // If stepping is enabled, make sure we skip those systems that should
        // not be run.
        #[cfg(feature = "bevy_debug_stepping")]
        if let Some(skipped_systems) = _skip_systems {
            debug_assert_eq!(skipped_systems.len(), state.completed_systems.len());
            // mark skipped systems as completed
            state.completed_systems |= skipped_systems;

            // signal the dependencies for each of the skipped systems, as
            // though they had run
            for system_index in skipped_systems.ones() {
                state.signal_dependents(system_index);
                state.ready_systems.remove(system_index);
            }
        }

        let thread_executor = world
            .get_resource::<MainThreadExecutor>()
            .map(|e| e.0.clone());
        let thread_executor = thread_executor.as_deref();

        let environment = &Environment::new(self, schedule, world);

        ComputeTaskPool::get_or_init(TaskPool::default).scope_with_executor(
            false,
            thread_executor,
            |scope| {
                let context = Context { environment, scope };

                // The first tick won't need to process finished systems, but we still need to run the loop in
                // tick_executor() in case a system completes while the first tick still holds the mutex.
                context.tick_executor();
            },
        );

        // End the borrows of self and world in environment by copying out the reference to systems.
        let systems = environment.systems;

        let state = self.state.get_mut().unwrap();
        if self.apply_final_deferred {
            // Do one final apply buffers after all systems have completed
            // Commands should be applied while on the scope's thread, not the executor's thread
            let res = apply_deferred(&state.unapplied_systems, systems, world);
            if let Err(payload) = res {
                let panic_payload = self.panic_payload.get_mut().unwrap();
                *panic_payload = Some(payload);
            }
            state.unapplied_systems.clear();
        }

        // check to see if there was a panic
        let payload = self.panic_payload.get_mut().unwrap();
        if let Some(payload) = payload.take() {
            std::panic::resume_unwind(payload);
        }

        debug_assert!(state.ready_systems.is_clear());
        debug_assert!(state.running_systems.is_clear());
        state.active_access.clear();
        state.evaluated_sets.clear();
        state.skipped_systems.clear();
        state.completed_systems.clear();
    }

    fn set_apply_final_deferred(&mut self, value: bool) {
        self.apply_final_deferred = value;
    }
}

impl<'scope, 'env: 'scope, 'sys> Context<'scope, 'env, 'sys> {
    fn system_completed(
        &self,
        system_index: usize,
        res: Result<(), Box<dyn Any + Send>>,
        system: &BoxedSystem,
    ) {
        // tell the executor that the system finished
        self.environment
            .executor
            .system_completion
            .push(SystemResult { system_index })
            .unwrap_or_else(|error| unreachable!("{}", error));
        if let Err(payload) = res {
            eprintln!("Encountered a panic in system `{}`!", &*system.name());
            // set the payload to propagate the error
            {
                let mut panic_payload = self.environment.executor.panic_payload.lock().unwrap();
                *panic_payload = Some(payload);
            }
        }
        self.tick_executor();
    }

    fn try_lock<'a>(&'a self) -> Option<(&'a mut Conditions<'sys>, MutexGuard<'a, ExecutorState>)> {
        let guard = self.environment.executor.state.try_lock().ok()?;
        // SAFETY: This is an exclusive access as no other location fetches conditions mutably, and
        // is synchronized by the lock on the executor state.
        let conditions = unsafe { &mut *self.environment.conditions.get() };
        Some((conditions, guard))
    }

    fn tick_executor(&self) {
        // Ensure that the executor handles any events pushed to the system_completion queue by this thread.
        // If this thread acquires the lock, the exector runs after the push() and they are processed.
        // If this thread does not acquire the lock, then the is_empty() check on the other thread runs
        // after the lock is released, which is after try_lock() failed, which is after the push()
        // on this thread, so the is_empty() check will see the new events and loop.
        loop {
            let Some((conditions, mut guard)) = self.try_lock() else {
                return;
            };
            guard.tick(self, conditions);
            // Make sure we drop the guard before checking system_completion.is_empty(), or we could lose events.
            drop(guard);
            if self.environment.executor.system_completion.is_empty() {
                return;
            }
        }
    }
}

impl MultiThreadedExecutor {
    /// Creates a new `multi_threaded` executor for use with a [`Schedule`].
    ///
    /// [`Schedule`]: crate::schedule::Schedule
    pub fn new() -> Self {
        Self {
            state: Mutex::new(ExecutorState::new()),
            system_completion: ConcurrentQueue::unbounded(),
            starting_systems: FixedBitSet::new(),
            apply_final_deferred: true,
            panic_payload: Mutex::new(None),
            #[cfg(feature = "trace")]
            executor_span: info_span!("multithreaded executor"),
        }
    }
}

impl ExecutorState {
    fn new() -> Self {
        Self {
            system_task_metadata: Vec::new(),
            num_running_systems: 0,
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
        }
    }

    fn tick(&mut self, context: &Context, conditions: &mut Conditions) {
        #[cfg(feature = "trace")]
        let _span = context.environment.executor.executor_span.enter();

        for result in context.environment.executor.system_completion.try_iter() {
            self.finish_system_and_handle_dependents(result);
        }

        self.rebuild_active_access();

        // SAFETY:
        // - `finish_system_and_handle_dependents` has updated the currently running systems.
        // - `rebuild_active_access` locks access for all currently running systems.
        unsafe {
            self.spawn_system_tasks(context, conditions);
        }
    }

    /// # Safety
    /// - Caller must ensure that `self.ready_systems` does not contain any systems that
    ///   have been mutably borrowed (such as the systems currently running).
    /// - `world_cell` must have permission to access all world data (not counting
    ///   any world data that is claimed by systems currently running on this executor).
    unsafe fn spawn_system_tasks(&mut self, context: &Context, conditions: &mut Conditions) {
        if self.exclusive_running {
            return;
        }

        // can't borrow since loop mutably borrows `self`
        let mut ready_systems = std::mem::take(&mut self.ready_systems_copy);

        // Skipping systems may cause their dependents to become ready immediately.
        // If that happens, we need to run again immediately or we may fail to spawn those dependents.
        let mut check_for_new_ready_systems = true;
        while check_for_new_ready_systems {
            check_for_new_ready_systems = false;

            ready_systems.clone_from(&self.ready_systems);

            for system_index in ready_systems.ones() {
                debug_assert!(!self.running_systems.contains(system_index));
                // SAFETY: Caller assured that these systems are not running.
                // Therefore, no other reference to this system exists and there is no aliasing.
                let system = unsafe { &mut *context.environment.systems[system_index].get() };

                if !self.can_run(
                    system_index,
                    system,
                    conditions,
                    context.environment.world_cell,
                ) {
                    // NOTE: exclusive systems with ambiguities are susceptible to
                    // being significantly displaced here (compared to single-threaded order)
                    // if systems after them in topological order can run
                    // if that becomes an issue, `break;` if exclusive system
                    continue;
                }

                self.ready_systems.remove(system_index);

                // SAFETY: `can_run` returned true, which means that:
                // - It must have called `update_archetype_component_access` for each run condition.
                // - There can be no systems running whose accesses would conflict with any conditions.
                if unsafe {
                    !self.should_run(
                        system_index,
                        system,
                        conditions,
                        context.environment.world_cell,
                    )
                } {
                    self.skip_system_and_signal_dependents(system_index);
                    // signal_dependents may have set more systems to ready.
                    check_for_new_ready_systems = true;
                    continue;
                }

                self.running_systems.insert(system_index);
                self.num_running_systems += 1;

                if self.system_task_metadata[system_index].is_exclusive {
                    // SAFETY: `can_run` returned true for this system,
                    // which means no systems are currently borrowed.
                    unsafe {
                        self.spawn_exclusive_system_task(context, system_index);
                    }
                    check_for_new_ready_systems = false;
                    break;
                }

                // SAFETY:
                // - Caller ensured no other reference to this system exists.
                // - `can_run` has been called, which calls `update_archetype_component_access` with this system.
                // - `can_run` returned true, so no systems with conflicting world access are running.
                unsafe {
                    self.spawn_system_task(context, system_index);
                }
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

            self.system_task_metadata[system_index]
                .archetype_component_access
                .clone_from(system.archetype_component_access());
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
            let set_conditions_met = unsafe {
                evaluate_and_fold_conditions(&mut conditions.set_conditions[set_idx], world)
            };

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
        let system_conditions_met = unsafe {
            evaluate_and_fold_conditions(&mut conditions.system_conditions[system_index], world)
        };

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
    unsafe fn spawn_system_task(&mut self, context: &Context, system_index: usize) {
        // SAFETY: this system is not running, no other reference exists
        let system = unsafe { &mut *context.environment.systems[system_index].get() };
        // Move the full context object into the new future.
        let context = *context;

        let system_meta = &self.system_task_metadata[system_index];

        let task = async move {
            let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                // SAFETY:
                // - The caller ensures that we have permission to
                // access the world data used by the system.
                // - `update_archetype_component_access` has been called.
                unsafe {
                    __rust_begin_short_backtrace::run_unsafe(
                        &mut **system,
                        context.environment.world_cell,
                    );
                };
            }));
            context.system_completed(system_index, res, system);
        };

        self.active_access
            .extend(&system_meta.archetype_component_access);

        if system_meta.is_send {
            context.scope.spawn(task);
        } else {
            self.local_thread_running = true;
            context.scope.spawn_on_external(task);
        }
    }

    /// # Safety
    /// Caller must ensure no systems are currently borrowed.
    unsafe fn spawn_exclusive_system_task(&mut self, context: &Context, system_index: usize) {
        // SAFETY: `can_run` returned true for this system, which means
        // that no other systems currently have access to the world.
        let world = unsafe { context.environment.world_cell.world_mut() };
        // SAFETY: this system is not running, no other reference exists
        let system = unsafe { &mut *context.environment.systems[system_index].get() };
        // Move the full context object into the new future.
        let context = *context;

        if is_apply_deferred(system) {
            // TODO: avoid allocation
            let unapplied_systems = self.unapplied_systems.clone();
            self.unapplied_systems.clear();
            let task = async move {
                let res = apply_deferred(&unapplied_systems, context.environment.systems, world);
                context.system_completed(system_index, res, system);
            };

            context.scope.spawn_on_scope(task);
        } else {
            let task = async move {
                let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    __rust_begin_short_backtrace::run(&mut **system, world);
                }));
                context.system_completed(system_index, res, system);
            };

            context.scope.spawn_on_scope(task);
        }

        self.exclusive_running = true;
        self.local_thread_running = true;
    }

    fn finish_system_and_handle_dependents(&mut self, result: SystemResult) {
        let SystemResult { system_index, .. } = result;

        if self.system_task_metadata[system_index].is_exclusive {
            self.exclusive_running = false;
        }

        if !self.system_task_metadata[system_index].is_send {
            self.local_thread_running = false;
        }

        debug_assert!(self.num_running_systems >= 1);
        self.num_running_systems -= 1;
        self.running_systems.remove(system_index);
        self.completed_systems.insert(system_index);
        self.unapplied_systems.insert(system_index);

        self.signal_dependents(system_index);
    }

    fn skip_system_and_signal_dependents(&mut self, system_index: usize) {
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
) -> Result<(), Box<dyn Any + Send>> {
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
            unsafe { __rust_begin_short_backtrace::readonly_run_unsafe(&mut **condition, world) }
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

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_ecs,
        prelude::Resource,
        schedule::{ExecutorKind, IntoSystemConfigs, Schedule},
        system::Commands,
        world::World,
    };

    #[derive(Resource)]
    struct R;

    #[test]
    fn skipped_systems_notify_dependents() {
        let mut world = World::new();
        let mut schedule = Schedule::default();
        schedule.set_executor_kind(ExecutorKind::MultiThreaded);
        schedule.add_systems(
            (
                (|| {}).run_if(|| false),
                // This system depends on a system that is always skipped.
                |mut commands: Commands| {
                    commands.insert_resource(R);
                },
            )
                .chain(),
        );
        schedule.run(&mut world);
        assert!(world.get_resource::<R>().is_some());
    }
}
