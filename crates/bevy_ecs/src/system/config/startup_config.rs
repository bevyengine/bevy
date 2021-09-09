use crate::{
    prelude::{ExclusiveSystem, IntoExclusiveSystem, IntoSystem, System},
    schedule::SystemSet,
    system::{AlreadyWasSystem, ExclusiveSystemCoerced, ExclusiveSystemFn},
};

use super::{ParallelSystemKind, SystemSetKind};

/// Allows configuration of a [System](super::System) to run at startup, optionally in a given [Stage](crate::schedule::Stage).
pub trait StartupConfig<Params, Configured> {
    fn startup(self) -> Configured;
}

impl<T, Params, Configured> StartupConfig<(ParallelSystemKind, Params), Configured> for T
where
    T: IntoSystem<(), (), Params, System = Configured>,
    Configured: System + IntoSystem<(), (), AlreadyWasSystem>,
{
    fn startup(self) -> Configured {
        let mut system = self.system();
        system.config_mut().startup();
        system
    }
}

impl StartupConfig<SystemSetKind, SystemSet> for SystemSet {
    fn startup(mut self) -> SystemSet {
        self.config_mut().startup();
        self
    }
}

impl<Params> StartupConfig<Params, ExclusiveSystemCoerced> for ExclusiveSystemCoerced {
    fn startup(mut self) -> ExclusiveSystemCoerced {
        self.config_mut().startup();
        self
    }
}

impl<T, Params> StartupConfig<Params, ExclusiveSystemFn> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemFn>,
{
    fn startup(self) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().startup();
        system
    }
}
