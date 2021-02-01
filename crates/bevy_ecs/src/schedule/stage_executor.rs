use std::ops::Range;

use bevy_tasks::{ComputeTaskPool, CountdownEvent, TaskPool};
use bevy_utils::tracing::trace;
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;

use crate::{
    ArchetypesGeneration, BoxedSystem, Resources, ThreadLocalExecution, TypeAccess, World,
};

pub trait SystemStageExecutor: Downcast + Send + Sync {
    fn execute_stage(
        &mut self,
        systems: &mut [BoxedSystem],
        changed_systems: &[usize],
        world: &mut World,
        resources: &mut Resources,
    );
}

impl_downcast!(SystemStageExecutor);

#[derive(Default)]
pub struct SerialSystemStageExecutor;

impl SystemStageExecutor for SerialSystemStageExecutor {
    fn execute_stage(
        &mut self,
        systems: &mut [BoxedSystem],
        _changed_systems: &[usize],
        world: &mut World,
        resources: &mut Resources,
    ) {
        for system in systems.iter_mut() {
            system.update(world);
            match system.thread_local_execution() {
                ThreadLocalExecution::NextFlush => {
                    system.run((), world, resources);
                }
                ThreadLocalExecution::Immediate => {
                    system.run((), world, resources);
                    system.run_thread_local(world, resources);
                }
            }
        }

        // "flush"
        for system in systems.iter_mut() {
            match system.thread_local_execution() {
                ThreadLocalExecution::NextFlush => system.run_thread_local(world, resources),
                ThreadLocalExecution::Immediate => { /* already ran immediate */ }
            }
        }
    }
}

/// Executes the stage in parallel by analyzing system dependencies.
/// System execution order is undefined except under the following conditions:
/// * systems in earlier stages run before systems in later stages
/// * in a given stage, systems that mutate [archetype+component] X cannot run before systems registered before them that read/write [archetype+component] X
/// * in a given stage, systems the read [archetype+component] X cannot run before systems registered before them that write [archetype+component] X
/// * in a given stage, systems that mutate resource Y cannot run before systems registered before them that read/write resource Y
/// * in a given stage, systems the read resource Y cannot run before systems registered before them that write resource Y
pub struct ParallelSystemStageExecutor {
    /// each system's set of dependencies
    system_dependencies: Vec<FixedBitSet>,
    /// count of each system's dependencies
    system_dependency_count: Vec<usize>,
    /// Countdown of finished dependencies, used to trigger the next system
    ready_events: Vec<Option<CountdownEvent>>,
    /// When a system finishes, it will decrement the countdown events of all dependents
    ready_events_of_dependents: Vec<Vec<CountdownEvent>>,
    /// each system's dependents (the systems that can't run until this system has run)
    system_dependents: Vec<Vec<usize>>,
    /// stores the indices of thread local systems in this stage, which are used during stage.prepare()
    thread_local_system_indices: Vec<usize>,
    /// When archetypes change a counter is bumped - we cache the state of that counter when it was
    /// last read here so that we can detect when archetypes are changed
    last_archetypes_generation: ArchetypesGeneration,
}

impl Default for ParallelSystemStageExecutor {
    fn default() -> Self {
        Self {
            system_dependents: Default::default(),
            system_dependency_count: Default::default(),
            ready_events: Default::default(),
            ready_events_of_dependents: Default::default(),
            system_dependencies: Default::default(),
            thread_local_system_indices: Default::default(),
            last_archetypes_generation: ArchetypesGeneration(u64::MAX), // MAX forces prepare to run the first time
        }
    }
}

impl ParallelSystemStageExecutor {
    pub fn system_dependents(&self) -> &[Vec<usize>] {
        &self.system_dependents
    }

    pub fn system_dependencies(&self) -> &[FixedBitSet] {
        &self.system_dependencies
    }

    /// Sets up state to run the next "batch" of systems. Each batch contains 0..n systems and
    /// optionally a thread local system at the end. After this function runs, a bunch of state
    /// in self will be populated for systems in this batch. Returns the range of systems
    /// that we prepared, up to but NOT including the thread local system that MIGHT be at the end
    /// of the range
    pub fn prepare_to_next_thread_local(
        &mut self,
        world: &World,
        systems: &mut [BoxedSystem],
        stage_changed: bool,
        next_thread_local_index: usize,
    ) -> Range<usize> {
        // Find the first system in this batch and (if there is one) the thread local system that
        // ends it.
        let (prepare_system_start_index, last_thread_local_index) = if next_thread_local_index == 0
        {
            (0, None)
        } else {
            // start right after the last thread local system
            (
                self.thread_local_system_indices[next_thread_local_index - 1] + 1,
                Some(self.thread_local_system_indices[next_thread_local_index - 1]),
            )
        };

        let prepare_system_index_range = if let Some(index) = self
            .thread_local_system_indices
            .get(next_thread_local_index)
        {
            // if there is an upcoming thread local system, prepare up to (and including) it
            prepare_system_start_index..(*index + 1)
        } else {
            // if there are no upcoming thread local systems, prepare everything right now
            prepare_system_start_index..systems.len()
        };

        let archetypes_generation_changed =
            self.last_archetypes_generation != world.archetypes_generation();

        if stage_changed || archetypes_generation_changed {
            // update each system's [archetype+component] access to latest world archetypes
            for system_index in prepare_system_index_range.clone() {
                systems[system_index].update(world);

                // Clear this so that the next block of code that populates it doesn't insert
                // duplicates
                self.system_dependents[system_index].clear();
                self.system_dependencies[system_index].clear();
            }

            // calculate dependencies between systems and build execution order
            let mut current_archetype_access = TypeAccess::default();
            let mut current_resource_access = TypeAccess::default();
            for system_index in prepare_system_index_range.clone() {
                let system = &systems[system_index];
                let archetype_access = system.archetype_component_access();
                match system.thread_local_execution() {
                    ThreadLocalExecution::NextFlush => {
                        let resource_access = system.resource_access();
                        // if any system before this one conflicts, check all systems that came before for compatibility
                        if !current_archetype_access.is_compatible(archetype_access)
                            || !current_resource_access.is_compatible(resource_access)
                        {
                            #[allow(clippy::needless_range_loop)]
                            for earlier_system_index in
                                prepare_system_index_range.start..system_index
                            {
                                let earlier_system = &systems[earlier_system_index];

                                // due to how prepare ranges work, previous systems should all be "NextFlush"
                                debug_assert_eq!(
                                    earlier_system.thread_local_execution(),
                                    ThreadLocalExecution::NextFlush
                                );

                                // if earlier system is incompatible, make the current system dependent
                                if !earlier_system
                                    .archetype_component_access()
                                    .is_compatible(archetype_access)
                                    || !earlier_system
                                        .resource_access()
                                        .is_compatible(resource_access)
                                {
                                    self.system_dependents[earlier_system_index].push(system_index);
                                    self.system_dependencies[system_index]
                                        .insert(earlier_system_index);
                                }
                            }
                        }

                        current_archetype_access.union(archetype_access);
                        current_resource_access.union(resource_access);

                        if let Some(last_thread_local_index) = last_thread_local_index {
                            self.system_dependents[last_thread_local_index].push(system_index);
                            self.system_dependencies[system_index].insert(last_thread_local_index);
                        }
                    }
                    ThreadLocalExecution::Immediate => {
                        for earlier_system_index in prepare_system_index_range.start..system_index {
                            // treat all earlier systems as "incompatible" to ensure we run this thread local system exclusively
                            self.system_dependents[earlier_system_index].push(system_index);
                            self.system_dependencies[system_index].insert(earlier_system_index);
                        }
                    }
                }
            }

            // Verify that dependents are not duplicated
            #[cfg(debug_assertions)]
            for system_index in prepare_system_index_range.clone() {
                let mut system_dependents_set = std::collections::HashSet::new();
                for dependent_system in &self.system_dependents[system_index] {
                    let inserted = system_dependents_set.insert(*dependent_system);

                    // This means duplicate values are in the system_dependents list
                    // This is reproducing when archetypes change. When we fix this, we can remove
                    // the hack below and make this a debug-only assert or remove it
                    debug_assert!(inserted);
                }
            }

            // Clear the ready events lists associated with each system so we can rebuild them
            for ready_events_of_dependents in
                &mut self.ready_events_of_dependents[prepare_system_index_range.clone()]
            {
                ready_events_of_dependents.clear();
            }

            // Now that system_dependents and system_dependencies is populated, update
            // system_dependency_count and ready_events
            for system_index in prepare_system_index_range.clone() {
                // Count all dependencies to update system_dependency_count
                assert!(!self.system_dependencies[system_index].contains(system_index));
                let dependency_count = self.system_dependencies[system_index].count_ones(..);
                self.system_dependency_count[system_index] = dependency_count;

                // If dependency count > 0, allocate a ready_event
                self.ready_events[system_index] = match self.system_dependency_count[system_index] {
                    0 => None,
                    dependency_count => Some(CountdownEvent::new(dependency_count as isize)),
                }
            }

            // Now that ready_events are created, we can build ready_events_of_dependents
            for system_index in prepare_system_index_range.clone() {
                for dependent_system in &self.system_dependents[system_index] {
                    self.ready_events_of_dependents[system_index].push(
                        self.ready_events[*dependent_system]
                            .as_ref()
                            .expect("A dependent task should have a non-None ready event.")
                            .clone(),
                    );
                }
            }
        } else {
            // Reset the countdown events for this range of systems. Resetting is required even if the
            // schedule didn't change
            self.reset_system_ready_events(prepare_system_index_range);
        }

        if let Some(index) = self
            .thread_local_system_indices
            .get(next_thread_local_index)
        {
            // if there is an upcoming thread local system, prepare up to (and NOT including) it
            prepare_system_start_index..(*index)
        } else {
            // if there are no upcoming thread local systems, prepare everything right now
            prepare_system_start_index..systems.len()
        }
    }

    fn reset_system_ready_events(&mut self, prepare_system_index_range: Range<usize>) {
        for system_index in prepare_system_index_range {
            let dependency_count = self.system_dependency_count[system_index];
            if dependency_count > 0 {
                self.ready_events[system_index]
                    .as_ref()
                    .expect("A system with >0 dependency count should have a non-None ready event.")
                    .reset(dependency_count as isize)
            }
        }
    }

    /// Runs the non-thread-local systems in the given prepared_system_range range
    pub fn run_systems(
        &self,
        world: &World,
        resources: &Resources,
        systems: &mut [BoxedSystem],
        prepared_system_range: Range<usize>,
        compute_pool: &TaskPool,
    ) {
        // Generate tasks for systems in the given range and block until they are complete
        trace!("running systems {:?}", prepared_system_range);
        compute_pool.scope(|scope| {
            let start_system_index = prepared_system_range.start;
            let mut system_index = start_system_index;
            for system in &mut systems[prepared_system_range] {
                trace!(
                    "prepare {} {} with {} dependents and {} dependencies",
                    system_index,
                    system.name(),
                    self.system_dependents[system_index].len(),
                    self.system_dependencies[system_index].count_ones(..)
                );

                // This event will be awaited, preventing the task from starting until all
                // our dependencies finish running
                let ready_event = &self.ready_events[system_index];

                // Clear any dependencies on systems before this range of systems. We know at this
                // point everything before start_system_index is finished, and our ready_event did
                // not exist to be decremented until we started processing this range
                if start_system_index != 0 {
                    if let Some(ready_event) = ready_event.as_ref() {
                        for dependency in self.system_dependencies[system_index].ones() {
                            if dependency < start_system_index {
                                ready_event.decrement();
                            }
                        }
                    }
                }

                let world_ref = &*world;
                let resources_ref = &*resources;

                let trigger_events = &self.ready_events_of_dependents[system_index];

                // Verify that any dependent task has a > 0 count. If a dependent task has > 0
                // count, then the current system we are starting now isn't blocking it from running
                // as it should be. Failure here implies the sync primitives are not matching the
                // intended schedule. This likely compiles out if trace/asserts are disabled but
                // make it explicitly debug-only anyways
                #[cfg(debug_assertions)]
                {
                    let dependent_systems = &self.system_dependents[system_index];
                    debug_assert_eq!(trigger_events.len(), dependent_systems.len());
                    for (trigger_event, dependent_system_index) in
                        trigger_events.iter().zip(dependent_systems)
                    {
                        debug_assert!(
                            *dependent_system_index < start_system_index || trigger_event.get() > 0
                        );
                    }
                }

                // Spawn the task
                scope.spawn(async move {
                    // Wait until our dependencies are done
                    if let Some(ready_event) = ready_event {
                        ready_event.listen().await;
                    }

                    // Execute the system - in a scope to ensure the system lock is dropped before
                    // triggering dependents
                    {
                        #[cfg(feature = "trace")]
                        let system_span = bevy_utils::tracing::info_span!(
                            "system",
                            name = system.name().as_ref()
                        );
                        #[cfg(feature = "trace")]
                        let _system_guard = system_span.enter();

                        // SAFETY: scheduler ensures safe world / resource access
                        unsafe {
                            system.run_unsafe((), world_ref, resources_ref);
                        }
                    }

                    // Notify dependents that this task is done
                    for trigger_event in trigger_events {
                        trigger_event.decrement();
                    }
                });
                system_index += 1;
            }
        });
    }
}

impl SystemStageExecutor for ParallelSystemStageExecutor {
    fn execute_stage(
        &mut self,
        systems: &mut [BoxedSystem],
        changed_systems: &[usize],
        world: &mut World,
        resources: &mut Resources,
    ) {
        let start_archetypes_generation = world.archetypes_generation();
        let compute_pool = resources
            .get_or_insert_with(|| ComputeTaskPool(TaskPool::default()))
            .clone();

        let stage_changed = !changed_systems.is_empty();

        // if the schedule has changed, clear executor state / fill it with new defaults
        // This is mostly zeroing out a bunch of arrays parallel to the systems array. They will get
        // repopulated by prepare_to_next_thread_local() calls
        if stage_changed {
            self.system_dependencies.clear();
            self.system_dependencies
                .resize_with(systems.len(), || FixedBitSet::with_capacity(systems.len()));

            self.system_dependency_count.clear();
            self.system_dependency_count.resize(systems.len(), 0);

            self.thread_local_system_indices = Vec::new();

            self.system_dependents.clear();
            self.system_dependents.resize(systems.len(), Vec::new());

            self.ready_events.resize(systems.len(), None);
            self.ready_events_of_dependents
                .resize(systems.len(), Vec::new());

            for (system_index, system) in systems.iter().enumerate() {
                if system.thread_local_execution() == ThreadLocalExecution::Immediate {
                    #[cfg(feature = "trace")]
                    let system_span =
                        bevy_utils::tracing::info_span!("system", name = system.name().as_ref());
                    #[cfg(feature = "trace")]
                    let _system_guard = system_span.enter();

                    self.thread_local_system_indices.push(system_index);
                }
            }
        }

        // index of next thread local system in thread_local_system_indices. (always incremented by one
        // when prepare_to_next_thread_local is called. (We prepared up to index 0 above)
        let mut next_thread_local_index = 0;

        {
            // Prepare all system up to and including the first thread local system. This will return
            // the range of systems to run, up to but NOT including the next thread local
            let prepared_system_range = self.prepare_to_next_thread_local(
                world,
                systems,
                stage_changed,
                next_thread_local_index,
            );

            // Run everything up to the thread local system
            self.run_systems(
                world,
                resources,
                systems,
                prepared_system_range,
                &*compute_pool,
            );
        }

        loop {
            // Bail if we have no more thread local systems
            if next_thread_local_index >= self.thread_local_system_indices.len() {
                break;
            }

            // Run the thread local system at the end of the range of systems we just processed
            let thread_local_system_index =
                self.thread_local_system_indices[next_thread_local_index];
            {
                // if a thread local system is ready to run, run it exclusively on the main thread
                let system = systems[thread_local_system_index].as_mut();

                #[cfg(feature = "trace")]
                let system_span = bevy_utils::tracing::info_span!(
                    "thread_local_system",
                    name = system.name().as_ref()
                );
                #[cfg(feature = "trace")]
                let _system_guard = system_span.enter();

                system.run((), world, resources);
                system.run_thread_local(world, resources);
            }

            // Now that the previous thread local system has run, time to advance to the next one
            next_thread_local_index += 1;

            // Prepare all systems up to and including the next thread local system. This will
            // return the range of systems to run, up to but NOT including the next thread local
            let run_ready_system_index_range = self.prepare_to_next_thread_local(
                world,
                systems,
                stage_changed,
                next_thread_local_index,
            );

            self.run_systems(
                world,
                resources,
                systems,
                run_ready_system_index_range,
                &*compute_pool,
            );
        }

        // "flush"
        for system in systems.iter_mut() {
            match system.thread_local_execution() {
                ThreadLocalExecution::NextFlush => {
                    #[cfg(feature = "trace")]
                    let system_span =
                        bevy_utils::tracing::info_span!("system", name = system.name().as_ref());
                    #[cfg(feature = "trace")]
                    let _system_guard = system_span.enter();
                    system.run_thread_local(world, resources);
                }
                ThreadLocalExecution::Immediate => { /* already ran */ }
            }
        }

        // If world's archetypes_generation is the same as it was before running any systems then
        // we can assume that all systems have correct archetype accesses.
        if start_archetypes_generation == world.archetypes_generation() {
            self.last_archetypes_generation = world.archetypes_generation();
        }
    }
}
