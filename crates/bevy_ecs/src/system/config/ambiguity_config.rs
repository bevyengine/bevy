use crate::{
    prelude::{ExclusiveSystem, IntoExclusiveSystem, IntoSystem, System},
    schedule::{AmbiguitySetLabel, SystemSet},
    system::{AlreadyWasSystem, ExclusiveSystemCoerced, ExclusiveSystemFn},
};

use super::{ParallelSystemKind, SystemSetKind};

/// Allows configuration of a [System](System)'s [ambiguity set](https://docs.rs/bevy/0.5.0/bevy/ecs/schedule/struct.ReportExecutionOrderAmbiguities.html).
pub trait AmbiguityConfig<Params, Configured> {
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> Configured;
}

impl<T, Params, Configured> AmbiguityConfig<(ParallelSystemKind, Params), Configured> for T
where
    T: IntoSystem<(), (), Params, System = Configured>,
    Configured: System + IntoSystem<(), (), AlreadyWasSystem>,
{
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> Configured {
        let mut system = self.system();
        system.config_mut().add_ambiguity_set(set);
        system
    }
}

impl AmbiguityConfig<SystemSetKind, SystemSet> for SystemSet {
    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.config_mut().add_ambiguity_set(set);
        self
    }
}

impl<Params> AmbiguityConfig<Params, ExclusiveSystemCoerced> for ExclusiveSystemCoerced {
    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.config_mut().add_ambiguity_set(set);
        self
    }
}

impl<T, Params> AmbiguityConfig<Params, ExclusiveSystemFn> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemFn>,
{
    fn in_ambiguity_set(self, set: impl AmbiguitySetLabel) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().add_ambiguity_set(set);
        system
    }
}
