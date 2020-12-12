#![allow(dead_code, unused_variables, unused_imports)]

use std::ops::Range;

use bevy_tasks::{ComputeTaskPool, CountdownEvent, Scope, TaskPool};
use bevy_utils::tracing::trace;
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;

use crate::{ArchetypesGeneration, Resources, System, SystemSet, TypeAccess, World};

pub trait SystemStageExecutor: Downcast + Send + Sync {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
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
        system_sets: &mut [SystemSet],
        world: &mut World,
        resources: &mut Resources,
    ) {
        for system_set in system_sets.iter_mut() {
            for system in system_set.systems_mut() {
                system.update(world);
                system.run((), world, resources);
            }
        }
        for system_set in system_sets.iter_mut() {
            for system in system_set.systems_mut() {
                // TODO support exclusive systems at start of stage
                system.run_exclusive(world, resources);
            }
        }
    }
}

/// Container for scheduling metadata associated with a system.
struct SystemSchedulingMetadata {
    /// Used to signal the system's task to start the system.
    notifier: CountdownEvent,
    /// Indices of systems that depend on this one, used to decrement their
    /// dependency counters when this system finishes.
    dependants: Vec<usize>,
    /// Total amount of dependencies this system has.
    dependencies_total: usize,
    /// Amount of unsatisfied dependencies, when it reaches 0 the system is queued to be started.
    dependencies_now: usize,
}

pub struct ParallelSystemStageExecutor {
    system_metadata: Vec<SystemSchedulingMetadata>,
    /// When archetypes change a counter is bumped - we cache the state of that counter when it was
    /// last read here so that we can detect when archetypes are changed
    last_archetypes_generation: ArchetypesGeneration,
}

impl Default for ParallelSystemStageExecutor {
    fn default() -> Self {
        Self {
            system_metadata: Default::default(),
            // MAX ensures metadata will be initialized on first run.
            last_archetypes_generation: ArchetypesGeneration(u64::MAX),
        }
    }
}

impl SystemStageExecutor for ParallelSystemStageExecutor {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        world: &mut World,
        resources: &mut Resources,
    ) {
        let compute_pool = resources
            .get_or_insert_with(|| ComputeTaskPool(TaskPool::default()))
            .clone();
        compute_pool.scope(|scope| {
            self.prepare(scope, world, resources);
        });
    }
}

impl ParallelSystemStageExecutor {
    fn prepare<'scope>(
        &mut self,
        scope: &mut Scope<'scope, ()>,
        world: &'scope World,
        resources: &'scope Resources,
    ) {
    }
}
