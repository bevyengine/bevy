use crate::{
    archetype::ArchetypeComponentId,
    query::Access,
    schedule_v3::{Runner, RunnerApplyBuffers, Schedule},
    system::MaybeUnsafeCell,
    world::World,
};

use bevy_tasks::{ComputeTaskPool, Scope, TaskPool};
#[cfg(feature = "trace")]
use bevy_utils::tracing::Instrument;

use async_channel::{Receiver, Sender};
use fixedbitset::FixedBitSet;

/// Per-system data used by the [`MultiThreadedRunner`].
struct SystemTaskMetadata {
    /// Notifies system task to start running.
    start_sender: Sender<()>,
    /// Receives the start signal from the runner.
    start_receiver: Receiver<()>,
    /// Indices of the systems that directly depend on the system.
    dependents: Vec<usize>,
    /// The number of dependencies the system has in total.
    dependencies_total: usize,
    /// The number of dependencies the system has that have not completed.
    dependencies_remaining: usize,
    /// The `ArchetypeComponentId` access of the system.
    // This exists because systems are borrowed while they're running, and we have to
    // clear and rebuild the runner's active access whenever systems complete.
    archetype_component_access: Access<ArchetypeComponentId>,
    /// Is `true` if the system accesses non-Send data.
    non_send_access: bool,
}

/// A [`Runner`] that runs systems on a task pool. Non-conflicting systems can run in parallel.
pub struct MultiThreadedRunner {
    /// Metadata for scheduling and running system tasks.
    system_task_metadata: Vec<SystemTaskMetadata>,
    /// Notifies runner that systems have completed.
    finish_sender: Sender<usize>,
    /// Receives system completion events.
    finish_receiver: Receiver<usize>,
    /// Union of the accesses of all currently running systems.
    active_archetype_component_access: Access<ArchetypeComponentId>,
    /// Is `true` if a system reading non-Send data is running.
    non_send_access_active: bool,
    /// System sets whose conditions have been evaluated or skipped.
    visited_sets: FixedBitSet,
    /// Systems that have no remaining dependencies and are waiting to run.
    ready_systems: FixedBitSet,
    /// Systems that are currently running.
    running_systems: FixedBitSet,
    /// Systems that have completed.
    completed_systems: FixedBitSet,
    /// Systems that have completed but have not had their buffers applied.
    unapplied_systems: FixedBitSet,
}

impl Default for MultiThreadedRunner {
    fn default() -> Self {
        let (finish_sender, finish_receiver) = async_channel::unbounded();
        Self {
            finish_sender,
            finish_receiver,
            non_send_access_active: false,
            ..Default::default()
        }
    }
}

impl Runner for MultiThreadedRunner {
    fn init(&mut self, schedule: &Schedule) {
        // pre-allocate space
        let sys_count = schedule.system_ids.len();
        let set_count = schedule.set_ids.len();

        self.visited_sets.grow(set_count);
        self.ready_systems.grow(sys_count);
        self.running_systems.grow(sys_count);
        self.completed_systems.grow(sys_count);
        self.unapplied_systems.grow(sys_count);

        self.system_task_metadata.clear();
        self.system_task_metadata
            .reserve(sys_count.saturating_sub(self.system_task_metadata.len()));

        for index in 0..sys_count {
            let (start_sender, start_receiver) = async_channel::bounded(1);
            let (num_dependencies, dependents) = schedule.system_deps[index].clone();

            self.system_task_metadata.push(SystemTaskMetadata {
                start_sender,
                start_receiver,
                dependents,
                dependencies_total: num_dependencies,
                dependencies_remaining: num_dependencies,
                non_send_access: !schedule.systems[index].borrow().is_send(),
                archetype_component_access: Default::default(),
            });
        }
    }

    fn run(&mut self, schedule: &mut Schedule, world: &mut World) {
        #[cfg(feature = "trace")]
        let _schedule_span = bevy_utils::tracing::info_span!("run_schedule").entered();
        world.init_resource::<RunnerApplyBuffers>();

        ComputeTaskPool::init(TaskPool::default).scope(|scope| {
            let mut runner = async {
                // systems with zero dependencies are ready
                for (index, system_meta) in self.system_task_metadata.iter_mut().enumerate() {
                    if system_meta.dependencies_total == 0 {
                        self.ready_systems.set(index, true);
                    }
                }

                let world = MaybeUnsafeCell::from_mut(world);
                while self.completed_systems.count_ones(..) != self.completed_systems.len() {
                    // start some ready systems
                    self.spawn_system_tasks(scope, schedule, world).await;

                    if self.running_systems.count_ones(..) != 0 {
                        #[cfg(feature = "trace")]
                        let await_span = bevy_utils::tracing::info_span!("await_tasks").entered();

                        // wait until one completes
                        let index = self
                            .finish_receiver
                            .recv()
                            .await
                            .unwrap_or_else(|error| unreachable!("{}", error));

                        #[cfg(feature = "trace")]
                        drop(await_span);

                        // one system completed
                        self.finish_system_and_signal_dependents(index);
                        // maybe more
                        while let Ok(index) = self.finish_receiver.try_recv() {
                            self.finish_system_and_signal_dependents(index);
                        }

                        // remove access of completed systems
                        self.active_archetype_component_access.clear();
                        for sys_idx in self.running_systems.ones() {
                            let system_meta = &self.system_task_metadata[sys_idx];
                            self.active_archetype_component_access
                                .extend(&system_meta.archetype_component_access);
                        }
                    }

                    if self.active_archetype_component_access.is_empty() {
                        // SAFETY: we have exclusive access to the world
                        let world = unsafe { &mut *world.into_cell_ref().get() };
                        self.check_apply_system_buffers(schedule, world);
                    }
                }
                debug_assert_eq!(self.ready_systems.count_ones(..), 0);
                debug_assert_eq!(self.running_systems.count_ones(..), 0);
                debug_assert_eq!(self.unapplied_systems.count_ones(..), 0);
                assert!(self.active_archetype_component_access.is_empty());

                self.visited_sets.clear();
                self.completed_systems.clear();
            };

            #[cfg(feature = "trace")]
            let runner_span = bevy_utils::tracing::info_span!("runner_task");
            #[cfg(feature = "trace")]
            let runner = runner.instrument(runner_span);
            scope.spawn(runner);
        });
    }
}

impl MultiThreadedRunner {
    pub fn new() -> Self {
        Self::default()
    }

    async fn spawn_system_tasks<'scope, 'world: 'scope>(
        &mut self,
        scope: &Scope<'_, 'scope, ()>,
        schedule: &'scope Schedule,
        world: MaybeUnsafeCell<'world, World>,
    ) {
        #[cfg(feature = "trace")]
        let span = bevy_utils::tracing::info_span!("spawn_system_tasks").entered();
        while let Some(index) = self.ready_systems.ones().next() {
            if !self.system_can_run(index, schedule, world) {
                continue;
            }

            if !self.system_should_run(index, schedule, world) {
                continue;
            }

            let system_meta = &self.system_task_metadata[index];
            let start_receiver = system_meta.start_receiver.clone();
            let finish_sender = self.finish_sender.clone();

            // SAFETY: no active references to this system
            let system = unsafe { schedule.systems[index].as_ptr().as_mut().unwrap() };

            #[cfg(feature = "trace")]
            let task_span = bevy_utils::tracing::info_span!("system_task", name = &*system.name());
            #[cfg(feature = "trace")]
            let system_span = bevy_utils::tracing::info_span!("system", name = &*system.name());

            let task = async move {
                start_receiver
                    .recv()
                    .await
                    .unwrap_or_else(|error| unreachable!("{}", error));

                #[cfg(feature = "trace")]
                let system_guard = system_span.enter();
                // SAFETY: does not conflict with currently running systems
                unsafe { system.run_unsafe((), world) };
                #[cfg(feature = "trace")]
                drop(system_guard);

                finish_sender
                    .send(index)
                    .await
                    .unwrap_or_else(|error| unreachable!("{}", error));
            };

            #[cfg(feature = "trace")]
            let task = task.instrument(task_span);

            if system_meta.non_send_access {
                scope.spawn_on_scope(task);
                self.non_send_access_active = true;
            } else {
                scope.spawn(task);
            }

            system_meta
                .start_sender
                .send(())
                .await
                .unwrap_or_else(|error| unreachable!("{}", error));

            self.active_archetype_component_access
                .extend(&system_meta.archetype_component_access);
            self.ready_systems.set(index, false);
            self.running_systems.set(index, true);
        }
    }

    /// Evaluates the access of the system at `index` to determine if it can start.
    fn system_can_run(
        &mut self,
        index: usize,
        schedule: &Schedule,
        world: MaybeUnsafeCell<World>,
    ) -> bool {
        #[cfg(feature = "trace")]
        // SAFETY: no active references to this system
        let system = schedule.systems[index].borrow();
        #[cfg(feature = "trace")]
        let _can_run_span =
            bevy_utils::tracing::info_span!("check_access", name = &*system.name()).entered();

        if self.active_archetype_component_access.has_write_all() {
            // exclusive system running
            return false;
        }

        let mut system = schedule.systems[index].borrow_mut();
        let system_meta = &self.system_task_metadata[index];
        if self.non_send_access_active && system_meta.non_send_access {
            // only one thread can access !Send data, so only one system can run
            return false;
        }

        // SAFETY: exclusive system not running
        let world = unsafe { world.into_ref() };
        system.update_archetype_component_access(world);

        let mut access = system.archetype_component_access().clone();
        let mut system_conditions = schedule.system_conditions[index].borrow_mut();
        for condition in system_conditions.iter_mut() {
            condition.update_archetype_component_access(world);
            access.extend(condition.archetype_component_access());
        }

        for set_idx in schedule.sets_of_systems[index].difference(&self.visited_sets) {
            let mut set_conditions = schedule.set_conditions[set_idx].borrow_mut();
            for condition in set_conditions.iter_mut() {
                condition.update_archetype_component_access(world);
                access.extend(condition.archetype_component_access());
            }
        }

        access.is_compatible(&self.active_archetype_component_access)
    }

    /// Evaluates the conditions of the system at `index` to determine if it should run.
    fn system_should_run(
        &mut self,
        index: usize,
        schedule: &Schedule,
        world: MaybeUnsafeCell<World>,
    ) -> bool {
        #[cfg(feature = "trace")]
        // SAFETY: no active references to this system
        let system = schedule.systems[index].borrow();
        #[cfg(feature = "trace")]
        let _should_run_span =
            bevy_utils::tracing::info_span!("check_conditions", name = &*system.name()).entered();

        let mut should_run = true;
        // evaluate the set conditions in hierarchical order
        for set_idx in schedule.sets_of_systems[index].ones() {
            if self.visited_sets.contains(set_idx) {
                continue;
            } else {
                self.visited_sets.set(set_idx, true);
            }

            let mut set_conditions = schedule.set_conditions[set_idx].borrow_mut();
            let set_conditions_met = set_conditions.iter_mut().all(|condition| {
                #[cfg(feature = "trace")]
                let _condition_span =
                    bevy_utils::tracing::info_span!("condition", name = &*condition.name())
                        .entered();
                // SAFETY: does not conflict with currently running systems
                unsafe { condition.run_unsafe((), world) }
            });

            if !set_conditions_met {
                // skip all descendant systems
                for sys_idx in schedule.systems_of_sets[set_idx].ones() {
                    // a system can be in more than one system set, only skip once
                    if !self.completed_systems.contains(sys_idx) {
                        self.skip_system_and_signal_dependents(sys_idx);
                    }
                }
            }

            should_run &= set_conditions_met;
        }

        if !should_run {
            // system was skipped above
            return false;
        }

        // evaluate the system's conditions
        let mut system_conditions = schedule.system_conditions[index].borrow_mut();
        should_run = system_conditions.iter_mut().all(|condition| {
            #[cfg(feature = "trace")]
            let _condition_span =
                bevy_utils::tracing::info_span!("condition", name = &*condition.name()).entered();
            // SAFETY: does not conflict with currently running systems
            unsafe { condition.run_unsafe((), world) }
        });

        if !should_run {
            self.skip_system_and_signal_dependents(index);
            return false;
        }

        true
    }

    fn finish_system_and_signal_dependents(&mut self, index: usize) {
        if self.system_task_metadata[index].non_send_access {
            self.non_send_access_active = false;
        }
        self.running_systems.set(index, false);
        self.completed_systems.set(index, true);
        self.unapplied_systems.set(index, true);
        self.signal_dependents(index);
    }

    fn skip_system_and_signal_dependents(&mut self, index: usize) {
        self.ready_systems.set(index, false);
        self.completed_systems.set(index, true);
        self.signal_dependents(index);
    }

    fn signal_dependents(&mut self, index: usize) {
        #[cfg(feature = "trace")]
        let _span = bevy_utils::tracing::info_span!("signal_dependents").entered();
        // TODO: Avoid cloning here without unsafe.
        let dependents = &self.system_task_metadata[index].dependents.clone();
        for &dep_idx in dependents.iter() {
            let dependent_meta = &mut self.system_task_metadata[dep_idx];
            dependent_meta.dependencies_remaining -= 1;
            if (dependent_meta.dependencies_remaining == 0)
                && !self.completed_systems.contains(dep_idx)
            {
                self.ready_systems.set(dep_idx, true);
            }
        }
    }

    fn check_apply_system_buffers(&mut self, schedule: &Schedule, world: &mut World) {
        let mut should_apply_buffers = world.resource_mut::<RunnerApplyBuffers>();
        if should_apply_buffers.0 {
            should_apply_buffers.0 = false;

            for sys_idx in self.unapplied_systems.ones() {
                let mut system = schedule.systems[sys_idx].borrow_mut();
                #[cfg(feature = "trace")]
                let _apply_buffers_span =
                    bevy_utils::tracing::info_span!("apply_buffers", name = &*system.name())
                        .entered();
                system.apply_buffers(world);
            }

            self.unapplied_systems.clear();
        }
    }
}
