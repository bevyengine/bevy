use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};
use fixedbitset::FixedBitSet;

use crate::{Resources, SystemIndex, SystemSet, World};

pub trait ParallelSystemExecutor: Downcast + Send + Sync {
    /// Runs the parallel systems in the given system sets,
    /// using the ordering rules provided by arguments.
    ///
    /// * `system_sets`: Groups of systems; each set has its own run criterion.
    /// * `dependency_graph`: Resolved graph of parallel systems and their dependencies.
    /// Contains all parallel systems.
    /// * `topological_order`: Topologically sorted parallel systems.
    /// * `system_should_run`: Indices of systems that should be ran, in topological order.
    #[allow(clippy::too_many_arguments)] // Hmm...
    fn run_systems(
        &mut self,
        system_sets: &mut [SystemSet],
        dependency_graph: &HashMap<SystemIndex, Vec<SystemIndex>>,
        topological_order: &[SystemIndex],
        system_should_run: &FixedBitSet,
        world: &mut World,
        resources: &mut Resources,
    );
}

impl_downcast!(ParallelSystemExecutor);

#[derive(Default)]
pub struct SingleThreadedExecutor;

impl ParallelSystemExecutor for SingleThreadedExecutor {
    fn run_systems(
        &mut self,
        system_sets: &mut [SystemSet],
        _dependency_graph: &HashMap<SystemIndex, Vec<SystemIndex>>,
        topological_order: &[SystemIndex],
        system_should_run: &FixedBitSet,
        world: &mut World,
        resources: &mut Resources,
    ) {
        for index in system_should_run.ones() {
            let index = topological_order[index];
            system_sets[index.set]
                .parallel_system_mut(index.system)
                .run((), world, resources);
        }
    }
}
