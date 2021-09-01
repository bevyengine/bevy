use crate::{prelude::{ExclusiveSystem, IntoExclusiveSystem, IntoSystem, System}, schedule::{SystemLabel, SystemSet}, system::{AlreadyWasSystem, ExclusiveSystemCoerced, ExclusiveSystemFn}};

use super::{ParallelSystemKind, SystemSetKind};

pub trait ScheduleConfig<Params, Configured> {
    fn label(self, label: impl SystemLabel) -> Configured;
    fn before(self, label: impl SystemLabel) -> Configured;
    fn after(self, label: impl SystemLabel) -> Configured;
}

impl<T, Params, Configured> ScheduleConfig<(ParallelSystemKind, Params), Configured> for T
where
    T: IntoSystem<(), (), Params, System = Configured>,
    Configured: System + IntoSystem<(), (), AlreadyWasSystem>,
{
    fn label(self, label: impl SystemLabel) -> Configured {
        let mut system = self.system();
        system.config_mut().add_label(label);
        system
    }
    fn before(self, label: impl SystemLabel) -> Configured {
        let mut system = self.system();
        system.config_mut().add_before(label);
        system
    }
    fn after(self, label: impl SystemLabel) -> Configured {
        let mut system = self.system();
        system.config_mut().add_after(label);
        system
    }
}

impl ScheduleConfig<SystemSetKind, SystemSet> for SystemSet {
    fn label(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_label(label);
        self
    }
    fn before(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_before(label);
        self
    }
    fn after(mut self, label: impl SystemLabel) -> Self {
        self.config_mut().add_after(label);
        self
    }
}

impl<Params, T> ScheduleConfig<Params, ExclusiveSystemCoerced> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemCoerced>,
{
    fn label(self, label: impl SystemLabel) -> ExclusiveSystemCoerced {
        let mut system = self.exclusive_system();
        system.config_mut().add_label(label);
        system
    }
    fn before(self, label: impl SystemLabel) -> ExclusiveSystemCoerced {
        let mut system = self.exclusive_system();
        system.config_mut().add_before(label);
        system
    }
    fn after(self, label: impl SystemLabel) -> ExclusiveSystemCoerced {
        let mut system = self.exclusive_system();
        system.config_mut().add_after(label);
        system
    }
}

impl<Params, T> ScheduleConfig<Params, ExclusiveSystemFn> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemFn>,
{
    fn label(self, label: impl SystemLabel) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().add_label(label);
        system
    }
    fn before(self, label: impl SystemLabel) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().add_before(label);
        system
    }
    fn after(self, label: impl SystemLabel) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().add_after(label);
        system
    }
}
