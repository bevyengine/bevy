use bevy_ecs::{
    prelude::{ExclusiveSystem, System},
    system::{ExclusiveSystemKind, ParallelSystemKind, StageConfig},
};

use crate::StartupStage;

trait StartupSystem<SystemKind> {
    fn startup(self) -> Self;
}

impl<T: System> StartupSystem<ParallelSystemKind> for T {
    fn startup(mut self) -> Self {
        self.stage(StartupStage::Startup)
    }
}

impl<T: ExclusiveSystem> StartupSystem<ExclusiveSystemKind> for T {
    fn startup(mut self) -> Self {
        self.stage(StartupStage::Startup)
    }
}
