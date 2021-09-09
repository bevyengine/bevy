use crate::{
    prelude::{ExclusiveSystem, IntoExclusiveSystem, IntoSystem, System},
    schedule::{IntoRunCriteria, SystemSet},
    system::{AlreadyWasSystem, ExclusiveSystemCoerced, ExclusiveSystemFn},
};

use super::{ParallelSystemKind, SystemSetKind};

/// Allows configuration of a [System](System) with a specific [RunCriteriaDescriptorOrLabel](crate::schedule::RunCriteriaDescriptorOrLabel).
pub trait RunCriteraConfig<Params, Configured> {
    fn with_run_criteria<Marker>(self, run_criteria: impl IntoRunCriteria<Marker>) -> Configured;
}

impl<T, Params, Configured> RunCriteraConfig<(ParallelSystemKind, Params), Configured> for T
where
    T: IntoSystem<(), (), Params, System = Configured>,
    Configured: System + IntoSystem<(), (), AlreadyWasSystem>,
{
    fn with_run_criteria<Marker>(self, run_criteria: impl IntoRunCriteria<Marker>) -> Configured {
        let mut system = self.system();
        system.config_mut().set_run_criteria(run_criteria.into());
        system
    }
}

impl RunCriteraConfig<SystemSetKind, SystemSet> for SystemSet {
    fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.config_mut().set_run_criteria(run_criteria.into());
        self
    }
}

impl<Params> RunCriteraConfig<Params, ExclusiveSystemCoerced> for ExclusiveSystemCoerced {
    fn with_run_criteria<Marker>(mut self, run_criteria: impl IntoRunCriteria<Marker>) -> Self {
        self.config_mut().set_run_criteria(run_criteria.into());
        self
    }
}

impl<T, Params> RunCriteraConfig<Params, ExclusiveSystemFn> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemFn>,
{
    fn with_run_criteria<Marker>(
        self,
        run_criteria: impl IntoRunCriteria<Marker>,
    ) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().set_run_criteria(run_criteria.into());
        system
    }
}
