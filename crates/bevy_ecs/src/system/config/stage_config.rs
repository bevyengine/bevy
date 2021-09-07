use crate::{
    prelude::{ExclusiveSystem, IntoExclusiveSystem, IntoSystem, System},
    schedule::{StageLabel, SystemSet},
    system::{AlreadyWasSystem, ExclusiveSystemCoerced, ExclusiveSystemFn},
};

use super::{ParallelSystemKind, SystemSetKind};

/// Allows configuration of a [System](System) to run in a specific [Stage](crate::schedule::Stage).
pub trait StageConfig<Params, Configured> {
    fn stage(self, label: impl StageLabel) -> Configured;
}

impl<T, Params, Configured> StageConfig<(ParallelSystemKind, Params), Configured> for T
where
    T: IntoSystem<(), (), Params, System = Configured>,
    Configured: System + IntoSystem<(), (), AlreadyWasSystem>,
{
    fn stage(self, label: impl StageLabel) -> Configured {
        let mut system = self.system();
        system.config_mut().set_stage(label);
        system
    }
}

impl StageConfig<SystemSetKind, SystemSet> for SystemSet {
    fn stage(mut self, label: impl StageLabel) -> SystemSet {
        self.config_mut().set_stage(label);
        self
    }
}

impl<Params> StageConfig<Params, ExclusiveSystemCoerced> for ExclusiveSystemCoerced {
    fn stage(mut self, label: impl StageLabel) -> ExclusiveSystemCoerced {
        self.config_mut().set_stage(label);
        self
    }
}

impl<T, Params> StageConfig<Params, ExclusiveSystemFn> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemFn>,
{
    fn stage(self, label: impl StageLabel) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().set_stage(label);
        system
    }
}
