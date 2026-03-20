use alloc::{boxed::Box, vec::Vec};
use bevy_platform::cell::SyncUnsafeCell;
use concurrent_queue::ConcurrentQueue;
use core::{any::Any, panic::AssertUnwindSafe};
use fixedbitset::FixedBitSet;
#[cfg(feature = "std")]
use std::eprintln;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Mutex,
    },
};

#[cfg(feature = "trace")]
use tracing::{info_span, Span};

use crate::{
    error::{ErrorContext, ErrorHandler, Result},
    schedule::{
        CompiledPlan, CompiledSystemLane, ConditionWithAccess, SystemExecutor, SystemSchedule,
        SystemWithAccess,
    },
    system::{RunSystemError, ScheduleSystem},
    world::{unsafe_world_cell::UnsafeWorldCell, World},
};
#[cfg(feature = "hotpatching")]
use crate::{prelude::DetectChanges, HotPatchChanges};

use super::__rust_begin_short_backtrace;

struct Environment<'env, 'sys> {
    systems: &'sys [SyncUnsafeCell<SystemWithAccess>],
    conditions: SyncUnsafeCell<Conditions<'sys>>,
    world_cell: UnsafeWorldCell<'env>,
}

struct Conditions<'a> {
    system_conditions: &'a mut [Vec<ConditionWithAccess>],
    set_conditions: &'a mut [Vec<ConditionWithAccess>],
    sets_with_conditions_of_systems: &'a [FixedBitSet],
    systems_in_sets_with_conditions: &'a [FixedBitSet],
}

impl<'env, 'sys> Environment<'env, 'sys> {
    fn new(schedule: &'sys mut SystemSchedule, world: &'env mut World) -> Self {
        Self {
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

#[derive(Default)]
struct WorkerQueue {
    queue: Mutex<VecDeque<usize>>,
}

impl WorkerQueue {
    fn push(&self, system_index: usize) {
        self.queue.lock().unwrap().push_back(system_index);
    }

    fn push_retry(&self, system_index: usize) {
        self.queue.lock().unwrap().push_front(system_index);
    }

    fn pop(&self) -> Option<usize> {
        self.queue.lock().unwrap().pop_back()
    }

    fn steal_half_into(&self, target: &WorkerQueue) -> bool {
        if core::ptr::eq(self, target) {
            return false;
        }

        let mut source = self.queue.lock().unwrap();
        let take = source.len() / 2;
        if take == 0 {
            return false;
        }

        let stolen: Vec<_> = source.drain(..take).collect();
        drop(source);

        let mut target = target.queue.lock().unwrap();
        for system_index in stolen {
            target.push_back(system_index);
        }

        true
    }
}

struct RunState {
    evaluated_sets: FixedBitSet,
    running_systems: FixedBitSet,
    completed_systems: FixedBitSet,
    skipped_systems: FixedBitSet,
    unapplied_systems: FixedBitSet,
    num_running_systems: usize,
    local_thread_running: bool,
    exclusive_running: bool,
}

enum ClaimResult {
    Run(CompiledSystemLane),
    Skipped,
    Retry,
    Done,
}

struct Runtime<'env, 'sys> {
    environment: &'env Environment<'env, 'sys>,
    compiled: CompiledPlan,
    state: Mutex<RunState>,
    worker_queues: Vec<WorkerQueue>,
    global_inject: ConcurrentQueue<usize>,
    main_queue: ConcurrentQueue<usize>,
    remaining_dependencies: Vec<AtomicUsize>,
    remaining_systems: AtomicUsize,
    stop: AtomicBool,
    apply_final_deferred: bool,
    panic_payload: Mutex<Option<Box<dyn Any + Send>>>,
    queue_injections: AtomicUsize,
    steals: AtomicUsize,
    conflict_retries: AtomicUsize,
    main_thread_claims: AtomicUsize,
    deferred_barriers: AtomicUsize,
    condition_evaluations: AtomicUsize,
    #[cfg(feature = "trace")]
    executor_span: Span,
}

/// Per-run counters collected by [`WorkStealingExecutor`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkStealingExecutorStats {
    /// Ready systems injected into a worker queue, the global queue, or the main-thread lane.
    pub queue_injections: usize,
    /// Successful half-queue steals performed by worker threads.
    pub steals: usize,
    /// Retries caused by lane or access conflicts after a system was dequeued.
    pub conflict_retries: usize,
    /// Systems claimed on the main-thread lane.
    pub main_thread_claims: usize,
    /// `ApplyDeferred` barriers executed during the run.
    pub deferred_barriers: usize,
    /// Individual run-condition evaluations performed during the run.
    pub condition_evaluations: usize,
}

impl<'env, 'sys> Runtime<'env, 'sys> {
    fn new(
        compiled: CompiledPlan,
        system_count: usize,
        set_count: usize,
        environment: &'env Environment<'env, 'sys>,
        worker_count: usize,
        apply_final_deferred: bool,
        skip_systems: Option<&FixedBitSet>,
    ) -> Self {
        let worker_count = worker_count.max(1);
        let worker_queues = (0..worker_count).map(|_| WorkerQueue::default()).collect();
        let remaining_dependencies = compiled
            .dependency_counts
            .iter()
            .copied()
            .map(AtomicUsize::new)
            .collect();

        let runtime = Self {
            environment,
            compiled,
            state: Mutex::new(RunState {
                evaluated_sets: FixedBitSet::with_capacity(set_count),
                running_systems: FixedBitSet::with_capacity(system_count),
                completed_systems: FixedBitSet::with_capacity(system_count),
                skipped_systems: FixedBitSet::with_capacity(system_count),
                unapplied_systems: FixedBitSet::with_capacity(system_count),
                num_running_systems: 0,
                local_thread_running: false,
                exclusive_running: false,
            }),
            worker_queues,
            global_inject: ConcurrentQueue::unbounded(),
            main_queue: ConcurrentQueue::unbounded(),
            remaining_dependencies,
            remaining_systems: AtomicUsize::new(system_count),
            stop: AtomicBool::new(false),
            apply_final_deferred,
            panic_payload: Mutex::new(None),
            queue_injections: AtomicUsize::new(0),
            steals: AtomicUsize::new(0),
            conflict_retries: AtomicUsize::new(0),
            main_thread_claims: AtomicUsize::new(0),
            deferred_barriers: AtomicUsize::new(0),
            condition_evaluations: AtomicUsize::new(0),
            #[cfg(feature = "trace")]
            executor_span: info_span!("work_stealing_executor"),
        };

        if let Some(skipped_systems) = skip_systems {
            for system_index in skipped_systems.ones() {
                runtime.skip_without_running(system_index, None);
            }
        }

        for system_index in runtime.compiled.starting_systems.ones() {
            runtime.enqueue_ready(system_index, None);
        }

        runtime
    }

    fn enqueue_ready(&self, system_index: usize, preferred_worker: Option<usize>) {
        if self.is_stopped() {
            return;
        }

        self.queue_injections.fetch_add(1, Ordering::Relaxed);

        match self.compiled.system_metadata[system_index].lane {
            CompiledSystemLane::Worker => {
                if let Some(worker_index) = preferred_worker {
                    let worker_index = worker_index % self.worker_queues.len();
                    self.worker_queues[worker_index].push(system_index);
                } else {
                    self.global_inject
                        .push(system_index)
                        .unwrap_or_else(|error| unreachable!("{}", error));
                }
            }
            CompiledSystemLane::MainThread
            | CompiledSystemLane::Exclusive
            | CompiledSystemLane::ApplyDeferred => {
                self.main_queue
                    .push(system_index)
                    .unwrap_or_else(|error| unreachable!("{}", error));
            }
        }
    }

    fn next_worker_system(&self, worker_index: usize, tick: usize) -> Option<usize> {
        if let Some(system_index) = self.worker_queues[worker_index].pop() {
            return Some(system_index);
        }

        if tick % 31 == 0
            && let Ok(system_index) = self.global_inject.pop()
        {
            return Some(system_index);
        }

        for offset in 1..self.worker_queues.len() {
            let victim = (worker_index + offset) % self.worker_queues.len();
            if self.worker_queues[victim].steal_half_into(&self.worker_queues[worker_index]) {
                self.steals.fetch_add(1, Ordering::Relaxed);
                return self.worker_queues[worker_index].pop();
            }
        }

        if let Ok(system_index) = self.global_inject.pop() {
            return Some(system_index);
        }

        None
    }

    fn next_main_thread_system(&self) -> Option<usize> {
        if let Ok(system_index) = self.main_queue.pop() {
            return Some(system_index);
        }

        if let Ok(system_index) = self.global_inject.pop() {
            return Some(system_index);
        }

        for worker in &self.worker_queues {
            if let Some(system_index) = worker.pop() {
                return Some(system_index);
            }
        }

        None
    }

    fn worker_loop(&self, worker_index: usize, error_handler: ErrorHandler) {
        #[cfg(feature = "trace")]
        let _span = self.executor_span.enter();

        let mut tick = 0usize;
        let mut retry_slot = None;
        loop {
            if self.is_finished() {
                return;
            }

            let Some(system_index) = self
                .next_worker_system(worker_index, tick)
                .or_else(|| retry_slot.take())
            else {
                std::thread::yield_now();
                tick = tick.wrapping_add(1);
                continue;
            };

            match self.try_claim_system(system_index, error_handler) {
                ClaimResult::Run(CompiledSystemLane::Worker) => {
                    self.execute_claimed_system(
                        system_index,
                        CompiledSystemLane::Worker,
                        error_handler,
                        Some(worker_index),
                    );
                }
                ClaimResult::Run(_) => {
                    unreachable!("worker threads only claim worker-lane systems")
                }
                ClaimResult::Skipped | ClaimResult::Done => {}
                ClaimResult::Retry => {
                    if retry_slot.is_none() {
                        retry_slot = Some(system_index);
                    } else {
                        self.worker_queues[worker_index].push_retry(system_index);
                    }
                    std::thread::yield_now();
                }
            }

            tick = tick.wrapping_add(1);
        }
    }

    fn main_thread_loop(&self, error_handler: ErrorHandler) {
        #[cfg(feature = "trace")]
        let _span = self.executor_span.enter();

        let mut retry_slot = None;
        while !self.is_finished() {
            let Some(system_index) = self.next_main_thread_system().or_else(|| retry_slot.take())
            else {
                std::thread::yield_now();
                continue;
            };

            match self.try_claim_system(system_index, error_handler) {
                ClaimResult::Run(lane) => {
                    self.execute_claimed_system(system_index, lane, error_handler, None);
                }
                ClaimResult::Skipped | ClaimResult::Done => {}
                ClaimResult::Retry => {
                    if let Some(previous) = retry_slot.replace(system_index) {
                        self.main_queue
                            .push(previous)
                            .unwrap_or_else(|error| unreachable!("{}", error));
                    }
                    std::thread::yield_now();
                }
            }
        }
    }

    fn is_finished(&self) -> bool {
        self.is_stopped() || self.remaining_systems.load(Ordering::Acquire) == 0
    }

    fn is_stopped(&self) -> bool {
        self.stop.load(Ordering::Acquire)
    }

    fn mark_panic(&self, payload: Box<dyn Any + Send>, system: &ScheduleSystem) {
        #[cfg(feature = "std")]
        #[expect(clippy::print_stderr, reason = "Allowed behind `std` feature gate.")]
        {
            eprintln!("Encountered a panic in system `{}`!", system.name());
        }

        *self.panic_payload.lock().unwrap() = Some(payload);
        self.stop.store(true, Ordering::Release);
    }

    fn try_claim_system(&self, system_index: usize, error_handler: ErrorHandler) -> ClaimResult {
        let lane = self.compiled.system_metadata[system_index].lane;
        let metadata = &self.compiled.system_metadata[system_index];
        let mut state = self.state.lock().unwrap();
        if state.completed_systems.contains(system_index)
            || state.running_systems.contains(system_index)
        {
            return ClaimResult::Done;
        }

        if self.remaining_dependencies[system_index].load(Ordering::Acquire) != 0 {
            return ClaimResult::Retry;
        }

        match lane {
            CompiledSystemLane::Worker => {
                if state.exclusive_running {
                    self.conflict_retries.fetch_add(1, Ordering::Relaxed);
                    return ClaimResult::Retry;
                }
            }
            CompiledSystemLane::MainThread => {
                if state.local_thread_running {
                    self.conflict_retries.fetch_add(1, Ordering::Relaxed);
                    return ClaimResult::Retry;
                }
            }
            CompiledSystemLane::Exclusive | CompiledSystemLane::ApplyDeferred => {
                if state.num_running_systems > 0 {
                    self.conflict_retries.fetch_add(1, Ordering::Relaxed);
                    return ClaimResult::Retry;
                }
            }
        }

        // SAFETY: Conditions are only accessed while holding the executor state lock.
        let conditions = unsafe { &mut *self.environment.conditions.get() };
        for set_idx in conditions.sets_with_conditions_of_systems[system_index]
            .difference(&state.evaluated_sets)
        {
            if !self.compiled.set_condition_conflicting_systems[set_idx]
                .is_disjoint(&state.running_systems)
            {
                self.conflict_retries.fetch_add(1, Ordering::Relaxed);
                return ClaimResult::Retry;
            }
        }

        if !metadata
            .condition_conflicting_systems
            .is_disjoint(&state.running_systems)
        {
            self.conflict_retries.fetch_add(1, Ordering::Relaxed);
            return ClaimResult::Retry;
        }

        if !state.skipped_systems.contains(system_index)
            && !metadata
                .conflicting_systems
                .is_disjoint(&state.running_systems)
        {
            self.conflict_retries.fetch_add(1, Ordering::Relaxed);
            return ClaimResult::Retry;
        }

        // SAFETY: The system is not running and mutable access is synchronized by the state lock.
        let system = unsafe { &mut *self.environment.systems[system_index].get() };

        #[cfg(feature = "hotpatching")]
        let hotpatch_tick = unsafe {
            self.environment
                .world_cell
                .get_resource_ref::<HotPatchChanges>()
        }
        .map(|r| r.last_changed())
        .unwrap_or_default();

        #[cfg(feature = "hotpatching")]
        if hotpatch_tick.is_newer_than(
            system.system.get_last_run(),
            self.environment.world_cell.change_tick(),
        ) {
            system.system.refresh_hotpatch();
        }

        if !self.should_run_locked(
            system_index,
            &mut system.system,
            conditions,
            &mut state,
            error_handler,
        ) {
            state.completed_systems.insert(system_index);
            drop(state);
            self.finish_without_running(system_index, None);
            return ClaimResult::Skipped;
        }

        state.running_systems.insert(system_index);
        state.num_running_systems += 1;

        match lane {
            CompiledSystemLane::Worker => {}
            CompiledSystemLane::MainThread => {
                state.local_thread_running = true;
                self.main_thread_claims.fetch_add(1, Ordering::Relaxed);
            }
            CompiledSystemLane::Exclusive | CompiledSystemLane::ApplyDeferred => {
                state.local_thread_running = true;
                state.exclusive_running = true;
                self.main_thread_claims.fetch_add(1, Ordering::Relaxed);
            }
        }

        ClaimResult::Run(lane)
    }

    fn should_run_locked(
        &self,
        system_index: usize,
        system: &mut ScheduleSystem,
        conditions: &mut Conditions,
        state: &mut RunState,
        error_handler: ErrorHandler,
    ) -> bool {
        let mut should_run = !state.skipped_systems.contains(system_index);

        for set_idx in conditions.sets_with_conditions_of_systems[system_index].ones() {
            if state.evaluated_sets.contains(set_idx) {
                continue;
            }

            self.condition_evaluations
                .fetch_add(conditions.set_conditions[set_idx].len(), Ordering::Relaxed);

            let set_conditions_met = unsafe {
                evaluate_and_fold_conditions(
                    &mut conditions.set_conditions[set_idx],
                    self.environment.world_cell,
                    error_handler,
                    system,
                    true,
                )
            };

            if !set_conditions_met {
                state
                    .skipped_systems
                    .union_with(&conditions.systems_in_sets_with_conditions[set_idx]);
            }

            should_run &= set_conditions_met;
            state.evaluated_sets.insert(set_idx);
        }

        self.condition_evaluations.fetch_add(
            conditions.system_conditions[system_index].len(),
            Ordering::Relaxed,
        );
        let system_conditions_met = unsafe {
            evaluate_and_fold_conditions(
                &mut conditions.system_conditions[system_index],
                self.environment.world_cell,
                error_handler,
                system,
                false,
            )
        };

        if !system_conditions_met {
            state.skipped_systems.insert(system_index);
        }

        should_run &= system_conditions_met;
        should_run
    }

    fn execute_claimed_system(
        &self,
        system_index: usize,
        lane: CompiledSystemLane,
        error_handler: ErrorHandler,
        preferred_worker: Option<usize>,
    ) {
        match lane {
            CompiledSystemLane::ApplyDeferred => {
                self.deferred_barriers.fetch_add(1, Ordering::Relaxed);
                let mut unapplied_systems = self.take_unapplied_systems();
                let world = unsafe { self.environment.world_cell.world_mut() };
                let system = unsafe { &mut *self.environment.systems[system_index].get() };
                let res = apply_deferred(&unapplied_systems, self.environment.systems, world);
                unapplied_systems.clear();
                self.restore_unapplied_systems(unapplied_systems);
                if let Err(payload) = res {
                    self.mark_panic(payload, &system.system);
                }
                self.finish_running(system_index, lane, preferred_worker);
            }
            CompiledSystemLane::Exclusive => {
                let world = unsafe { self.environment.world_cell.world_mut() };
                let system = unsafe { &mut *self.environment.systems[system_index].get() };
                let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
                    if let Err(RunSystemError::Failed(err)) =
                        __rust_begin_short_backtrace::run(&mut system.system, world)
                    {
                        error_handler(
                            err,
                            ErrorContext::System {
                                name: system.system.name(),
                                last_run: system.system.get_last_run(),
                            },
                        );
                    }
                }));
                if let Err(payload) = res {
                    self.mark_panic(payload, &system.system);
                }
                self.finish_running(system_index, lane, preferred_worker);
            }
            CompiledSystemLane::Worker | CompiledSystemLane::MainThread => {
                let system = unsafe { &mut *self.environment.systems[system_index].get() };
                let res = std::panic::catch_unwind(AssertUnwindSafe(|| unsafe {
                    if let Err(RunSystemError::Failed(err)) =
                        __rust_begin_short_backtrace::run_unsafe(
                            &mut system.system,
                            self.environment.world_cell,
                        )
                    {
                        error_handler(
                            err,
                            ErrorContext::System {
                                name: system.system.name(),
                                last_run: system.system.get_last_run(),
                            },
                        );
                    }
                }));
                if let Err(payload) = res {
                    self.mark_panic(payload, &system.system);
                }
                self.finish_running(system_index, lane, preferred_worker);
            }
        }
    }

    fn take_unapplied_systems(&self) -> FixedBitSet {
        let mut state = self.state.lock().unwrap();
        core::mem::take(&mut state.unapplied_systems)
    }

    fn restore_unapplied_systems(&self, unapplied_systems: FixedBitSet) {
        let mut state = self.state.lock().unwrap();
        debug_assert!(state.unapplied_systems.is_clear());
        state.unapplied_systems = unapplied_systems;
    }

    fn finish_running(
        &self,
        system_index: usize,
        lane: CompiledSystemLane,
        preferred_worker: Option<usize>,
    ) {
        {
            let mut state = self.state.lock().unwrap();
            state.running_systems.remove(system_index);
            state.completed_systems.insert(system_index);
            state.unapplied_systems.insert(system_index);
            state.num_running_systems = state.num_running_systems.saturating_sub(1);

            match lane {
                CompiledSystemLane::Worker => {}
                CompiledSystemLane::MainThread => {
                    state.local_thread_running = false;
                }
                CompiledSystemLane::Exclusive | CompiledSystemLane::ApplyDeferred => {
                    state.local_thread_running = false;
                    state.exclusive_running = false;
                }
            }
        }

        self.signal_completion(system_index, preferred_worker);
    }

    fn finish_without_running(&self, system_index: usize, preferred_worker: Option<usize>) {
        self.signal_completion(system_index, preferred_worker);
    }

    fn signal_completion(&self, system_index: usize, preferred_worker: Option<usize>) {
        if self.remaining_systems.fetch_sub(1, Ordering::AcqRel) == 1 {
            return;
        }

        for &dependent in self.compiled.dependents(system_index) {
            let remaining = self.remaining_dependencies[dependent].fetch_sub(1, Ordering::AcqRel);
            debug_assert!(remaining >= 1);
            if remaining == 1 {
                self.enqueue_ready(dependent, preferred_worker);
            }
        }
    }

    fn skip_without_running(&self, system_index: usize, preferred_worker: Option<usize>) {
        {
            let mut state = self.state.lock().unwrap();
            if state.completed_systems.contains(system_index) {
                return;
            }
            state.completed_systems.insert(system_index);
            state.skipped_systems.insert(system_index);
        }
        self.finish_without_running(system_index, preferred_worker);
    }

    fn apply_final_deferred(&self) -> Result<(), Box<dyn Any + Send>> {
        if !self.apply_final_deferred {
            return Ok(());
        }

        let mut unapplied_systems = self.take_unapplied_systems();

        let world = unsafe { self.environment.world_cell.world_mut() };
        let res = apply_deferred(&unapplied_systems, self.environment.systems, world);
        unapplied_systems.clear();
        self.restore_unapplied_systems(unapplied_systems);
        res
    }

    fn snapshot_stats(&self) -> WorkStealingExecutorStats {
        WorkStealingExecutorStats {
            queue_injections: self.queue_injections.load(Ordering::Relaxed),
            steals: self.steals.load(Ordering::Relaxed),
            conflict_retries: self.conflict_retries.load(Ordering::Relaxed),
            main_thread_claims: self.main_thread_claims.load(Ordering::Relaxed),
            deferred_barriers: self.deferred_barriers.load(Ordering::Relaxed),
            condition_evaluations: self.condition_evaluations.load(Ordering::Relaxed),
        }
    }
}

/// Runs the schedule using worker-local queues and a global inject queue.
pub struct WorkStealingExecutor {
    apply_final_deferred: bool,
    last_stats: WorkStealingExecutorStats,
}

impl Default for WorkStealingExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkStealingExecutor {
    /// Creates a new work-stealing executor for use with a [`Schedule`](crate::schedule::Schedule).
    pub const fn new() -> Self {
        Self {
            apply_final_deferred: true,
            last_stats: WorkStealingExecutorStats {
                queue_injections: 0,
                steals: 0,
                conflict_retries: 0,
                main_thread_claims: 0,
                deferred_barriers: 0,
                condition_evaluations: 0,
            },
        }
    }

    /// Returns counters collected during the most recent run.
    pub fn last_stats(&self) -> WorkStealingExecutorStats {
        self.last_stats
    }
}

impl SystemExecutor for WorkStealingExecutor {
    fn init(&mut self, _schedule: &SystemSchedule) {}

    fn run(
        &mut self,
        schedule: &mut SystemSchedule,
        world: &mut World,
        skip_systems: Option<&FixedBitSet>,
        error_handler: ErrorHandler,
    ) {
        if schedule.systems.is_empty() {
            self.last_stats = WorkStealingExecutorStats::default();
            return;
        }

        let worker_count = bevy_tasks::ComputeTaskPool::get_or_init(bevy_tasks::TaskPool::default)
            .thread_num()
            .max(1);
        let compiled = schedule.compiled.clone();
        let system_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();
        let environment = Environment::new(schedule, world);
        let runtime = Runtime::new(
            compiled,
            system_count,
            set_count,
            &environment,
            worker_count,
            self.apply_final_deferred,
            skip_systems,
        );

        bevy_tasks::ComputeTaskPool::get().scope(|scope| {
            for worker_index in 0..worker_count {
                let runtime = &runtime;
                scope.spawn(async move {
                    runtime.worker_loop(worker_index, error_handler);
                });
            }
            runtime.main_thread_loop(error_handler);
        });

        let payload = runtime.panic_payload.lock().unwrap().take();
        self.last_stats = runtime.snapshot_stats();
        let res = runtime.apply_final_deferred();
        if let Err(payload) = res {
            std::panic::resume_unwind(payload);
        }
        if let Some(payload) = payload {
            std::panic::resume_unwind(payload);
        }
    }

    fn set_apply_final_deferred(&mut self, value: bool) {
        self.apply_final_deferred = value;
    }
}

fn apply_deferred(
    unapplied_systems: &FixedBitSet,
    systems: &[SyncUnsafeCell<SystemWithAccess>],
    world: &mut World,
) -> Result<(), Box<dyn Any + Send>> {
    for system_index in unapplied_systems.ones() {
        let system = &mut unsafe { &mut *systems[system_index].get() }.system;
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
            system.apply_deferred(world);
        }));
        if let Err(payload) = res {
            #[cfg(feature = "std")]
            #[expect(clippy::print_stderr, reason = "Allowed behind `std` feature gate.")]
            {
                eprintln!(
                    "Encountered a panic when applying buffers for system `{}`!",
                    system.name()
                );
            }
            return Err(payload);
        }
    }
    Ok(())
}

unsafe fn evaluate_and_fold_conditions(
    conditions: &mut [ConditionWithAccess],
    world: UnsafeWorldCell,
    error_handler: ErrorHandler,
    for_system: &ScheduleSystem,
    on_set: bool,
) -> bool {
    #[expect(
        clippy::unnecessary_fold,
        reason = "Short-circuiting here would prevent conditions from mutating their own state as needed."
    )]
    conditions
        .iter_mut()
        .map(|ConditionWithAccess { condition, .. }| {
            unsafe { __rust_begin_short_backtrace::readonly_run_unsafe(&mut **condition, world) }
                .unwrap_or_else(|err| {
                    if let RunSystemError::Failed(err) = err {
                        error_handler(
                            err,
                            ErrorContext::RunCondition {
                                name: condition.name(),
                                last_run: condition.get_last_run(),
                                system: for_system.name(),
                                on_set,
                            },
                        );
                    };
                    false
                })
        })
        .fold(true, |acc, res| acc && res)
}
