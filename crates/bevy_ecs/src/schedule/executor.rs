use crate::{schedule::SystemContainer, world::World};
use core::fmt::Debug;
use downcast_rs::{impl_downcast, Downcast};

pub trait ParallelSystemExecutor: Downcast + Send + Sync {
    /// Called by `SystemStage` whenever `systems` have been changed.
    fn rebuild_cached_data(&mut self, systems: &[SystemContainer]);

    fn run_systems(&mut self, systems: &mut [SystemContainer], world: &mut World);
}

impl Debug for dyn ParallelSystemExecutor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "dyn ParallelSystemExecutor")
    }
}

impl_downcast!(ParallelSystemExecutor);

#[derive(Debug, Default)]
pub struct SingleThreadedExecutor;

impl ParallelSystemExecutor for SingleThreadedExecutor {
    fn rebuild_cached_data(&mut self, _: &[SystemContainer]) {}

    fn run_systems(&mut self, systems: &mut [SystemContainer], world: &mut World) {
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
