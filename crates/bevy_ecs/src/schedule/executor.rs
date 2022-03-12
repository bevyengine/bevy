use crate::{schedule::ParallelSystemContainer, world::World};
use downcast_rs::{impl_downcast, Downcast};

pub trait ParallelSystemExecutor: Downcast + Send + Sync {
    /// Called by `SystemStage` whenever `systems` have been changed.
    fn rebuild_cached_data(&mut self, systems: &[ParallelSystemContainer]);

    fn run_systems(&mut self, systems: &mut [ParallelSystemContainer], world: &mut World);
}

impl_downcast!(ParallelSystemExecutor);

#[derive(Default)]
pub struct SingleThreadedExecutor;

impl ParallelSystemExecutor for SingleThreadedExecutor {
    fn rebuild_cached_data(&mut self, _: &[ParallelSystemContainer]) {}

    fn run_systems(&mut self, systems: &mut [ParallelSystemContainer], world: &mut World) {
        for system in systems {
            if system.should_run() {
                #[cfg(feature = "trace")]
                let _system_span =
                    bevy_utils::tracing::info_span!("system", name = &*system.name()).entered();
                system.system_mut().run((), world);
            }
        }
    }
}
