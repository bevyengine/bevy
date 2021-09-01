use crate::{prelude::{ExclusiveSystem, IntoSystem, System}, schedule::{AmbiguitySetLabel, SystemSet}, system::{AlreadyWasSystem, ExclusiveSystemCoerced, ExclusiveSystemFn}};

use super::{ParallelSystemKind, SystemSetKind};

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

impl<Params> AmbiguityConfig<Params, ExclusiveSystemFn> for ExclusiveSystemFn {
    fn in_ambiguity_set(mut self, set: impl AmbiguitySetLabel) -> Self {
        self.config_mut().add_ambiguity_set(set);
        self
    }
}
