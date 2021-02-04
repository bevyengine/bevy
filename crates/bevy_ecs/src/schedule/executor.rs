use downcast_rs::{impl_downcast, Downcast};

use crate::{ParallelSystemContainer, Resources, World};

pub trait ParallelSystemExecutor: Downcast + Send + Sync {
    /// Called by `SystemStage` whenever `systems` have been changed.
    fn rebuild_cached_data(&mut self, systems: &mut [ParallelSystemContainer], world: &World);

    fn run_systems(
        &mut self,
        systems: &mut [ParallelSystemContainer],
        world: &mut World,
        resources: &mut Resources,
    );
}

impl_downcast!(ParallelSystemExecutor);

#[derive(Default)]
pub struct SingleThreadedExecutor;

impl ParallelSystemExecutor for SingleThreadedExecutor {
    fn rebuild_cached_data(&mut self, _: &mut [ParallelSystemContainer], _: &World) {}

    fn run_systems(
        &mut self,
        systems: &mut [ParallelSystemContainer],
        world: &mut World,
        resources: &mut Resources,
    ) {
        for system in systems {
            if system.should_run() {
                system.system_mut().run((), world, resources);
            }
        }
    }
}
