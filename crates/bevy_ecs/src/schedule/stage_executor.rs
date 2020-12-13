#![allow(dead_code, unused_variables, unused_imports)]

use std::ops::Range;

use bevy_tasks::{ComputeTaskPool, CountdownEvent, Scope, TaskPool};
use bevy_utils::{tracing::trace, HashMap};
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;

use crate::{ArchetypesGeneration, Resources, System, SystemIndex, SystemSet, TypeAccess, World};

type Label = &'static str; // TODO

pub trait SystemStageExecutor: Downcast + Send + Sync {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        system_labels: &HashMap<Label, SystemIndex>,
        world: &mut World,
        resources: &mut Resources,
    );
}

impl_downcast!(SystemStageExecutor);

pub struct SerialSystemStageExecutor {
    // Determines if a system has had its exclusive part already executed.
    exclusive_ran: FixedBitSet,
    last_archetypes_generation: ArchetypesGeneration,
}

impl Default for SerialSystemStageExecutor {
    fn default() -> Self {
        Self {
            exclusive_ran: FixedBitSet::with_capacity(64),
            // MAX ensures metadata will be initialized on first run.
            last_archetypes_generation: ArchetypesGeneration(u64::MAX),
        }
    }
}

impl SystemStageExecutor for SerialSystemStageExecutor {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        system_labels: &HashMap<Label, SystemIndex>,
        world: &mut World,
        resources: &mut Resources,
    ) {
        self.exclusive_ran.clear();
        let mut index = 0;
        for system_set in system_sets.iter_mut() {
            self.exclusive_ran.grow(index + system_set.systems().len());
            for system_index in 0..system_set.systems().len() {
                // TODO handle order of operations set by dependencies.
                let is_exclusive = {
                    let system = &system_set.systems()[system_index];
                    system.archetype_component_access().writes_all()
                        || system.resource_access().writes_all()
                };
                if is_exclusive {
                    system_set.systems_mut()[system_index].run_exclusive(world, resources);
                    self.exclusive_ran.set(index, true);
                }
                index += 1;
            }
        }
        if self.last_archetypes_generation != world.archetypes_generation() {
            for system_set in system_sets.iter_mut() {
                for system in system_set.systems_mut() {
                    system.update_access(world);
                    system.run((), world, resources);
                }
            }
            self.last_archetypes_generation = world.archetypes_generation();
        } else {
            for system_set in system_sets.iter_mut() {
                system_set.for_each_changed_system(|system| system.update_access(world));
                for system in system_set.systems_mut() {
                    system.run((), world, resources);
                }
            }
        }
        let mut index = 0;
        for system_set in system_sets.iter_mut() {
            for system in system_set.systems_mut() {
                if !self.exclusive_ran[index] {
                    system.run_exclusive(world, resources);
                }
                index += 1;
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
    dependants: Vec<SystemIndex>,
    /// Total amount of dependencies this system has.
    dependencies_total: usize,
    /// Amount of unsatisfied dependencies, when it reaches 0 the system is queued to be started.
    dependencies_now: usize,
}

pub struct ParallelSystemStageExecutor {
    system_metadata: Vec<Vec<SystemSchedulingMetadata>>,
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

impl ParallelSystemStageExecutor {
    fn metadata(&self, index: SystemIndex) -> &SystemSchedulingMetadata {
        &self.system_metadata[index.set][index.system]
    }

    fn prepare<'scope>(
        &mut self,
        scope: &mut Scope<'scope, ()>,
        system_sets: &'scope mut [SystemSet],
        world: &'scope World,
        resources: &'scope Resources,
    ) {
        for system_set in system_sets.iter_mut() {
            for system in system_set.systems_mut() {}
        }
        if self.last_archetypes_generation != world.archetypes_generation() {
            self.last_archetypes_generation = world.archetypes_generation();
        }
    }
}

impl SystemStageExecutor for ParallelSystemStageExecutor {
    fn execute_stage(
        &mut self,
        system_sets: &mut [SystemSet],
        system_labels: &HashMap<Label, SystemIndex>,
        world: &mut World,
        resources: &mut Resources,
    ) {
        let compute_pool = resources
            .get_or_insert_with(|| ComputeTaskPool(TaskPool::default()))
            .clone();
        compute_pool.scope(|scope| {
            self.prepare(scope, system_sets, world, resources);
        });
    }
}
