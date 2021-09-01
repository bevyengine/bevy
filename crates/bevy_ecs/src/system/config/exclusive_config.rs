use crate::{
    prelude::{ExclusiveSystem, IntoExclusiveSystem},
    system::{ExclusiveSystemCoerced, ExclusiveSystemFn, InsertionPoint},
};

pub trait ExclusiveConfig<Params, Configured> {
    fn at_start(self) -> Configured;
    fn before_commands(self) -> Configured;
    fn at_end(self) -> Configured;
}

impl<Params, T> ExclusiveConfig<Params, ExclusiveSystemCoerced> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemCoerced>,
{
    fn at_start(self) -> ExclusiveSystemCoerced {
        let mut system = self.exclusive_system();
        system.config_mut().insertion_point = Some(InsertionPoint::AtStart);
        system
    }
    fn before_commands(self) -> ExclusiveSystemCoerced {
        let mut system = self.exclusive_system();
        system.config_mut().insertion_point = Some(InsertionPoint::BeforeCommands);
        system
    }
    fn at_end(self) -> ExclusiveSystemCoerced {
        let mut system = self.exclusive_system();
        system.config_mut().insertion_point = Some(InsertionPoint::AtEnd);
        system
    }
}

impl<Params, T> ExclusiveConfig<Params, ExclusiveSystemFn> for T
where
    T: IntoExclusiveSystem<Params, ExclusiveSystemFn>,
{
    fn at_start(self) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().insertion_point = Some(InsertionPoint::AtStart);
        system
    }
    fn before_commands(self) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().insertion_point = Some(InsertionPoint::BeforeCommands);
        system
    }
    fn at_end(self) -> ExclusiveSystemFn {
        let mut system = self.exclusive_system();
        system.config_mut().insertion_point = Some(InsertionPoint::AtEnd);
        system
    }
}
