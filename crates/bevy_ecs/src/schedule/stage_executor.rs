use bevy_utils::HashMap;
use downcast_rs::{impl_downcast, Downcast};

use crate::{
    Resources,
    ShouldRun::{self, *},
    SystemIndex, SystemSet, World,
};

pub trait ParallelSystemExecutor: Downcast + Send + Sync {
    /// Runs the parallel systems in the given system sets,
    /// using the ordering rules provided by arguments.
    ///
    /// * `system_sets`: Groups of systems; each set has its own run criterion.
    /// * `dependency_graph`: Resolved graph of parallel systems and their dependencies.
    /// Contains all parallel systems.
    /// * `topological_order`: Topologically sorted parallel systems.
    #[allow(clippy::too_many_arguments)] // Hmm...
    fn run_systems(
        &mut self,
        system_sets: &mut [SystemSet],
        system_set_should_run: &[ShouldRun],
        dependency_graph: &HashMap<SystemIndex, Vec<SystemIndex>>,
        topological_order: &[SystemIndex],
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
        system_set_should_run: &[ShouldRun],
        _dependency_graph: &HashMap<SystemIndex, Vec<SystemIndex>>,
        topological_order: &[SystemIndex],
        world: &mut World,
        resources: &mut Resources,
    ) {
        for index in topological_order {
            if let Yes | YesAndLoop = system_set_should_run[index.set] {
                system_sets[index.set]
                    .parallel_system_mut(index.system)
                    .run((), world, resources);
            }
        }
    }
}
