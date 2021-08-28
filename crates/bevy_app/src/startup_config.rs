use bevy_ecs::{
    prelude::{ExclusiveSystem, System},
    system::{ExclusiveSystemKind, ParallelSystemKind, StageConfig},
};

use crate::StartupStage;

trait StartupConfig<SystemKind> {
    fn startup(self) -> Self;
}

impl<T: System> StartupConfig<ParallelSystemKind> for T {
    fn startup(mut self) -> Self {
        self.stage(StartupStage::Startup)
    }
}

impl<T: ExclusiveSystem> StartupConfig<ExclusiveSystemKind> for T {
    fn startup(mut self) -> Self {
        self.stage(StartupStage::Startup)
    }
}
