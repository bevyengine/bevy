use crate::{prelude::{ExclusiveSystem, IntoExclusiveSystem}, system::{ExclusiveSystemCoerced, ExclusiveSystemFn, InsertionPoint}};

pub trait ExclusiveConfig<Params, Configured> {
    fn at_start(self) -> Configured;
    fn before_commands(self) -> Configured;
    fn at_end(self) -> Configured;
}

impl<Params> ExclusiveConfig<Params, ExclusiveSystemCoerced> for ExclusiveSystemCoerced {
    fn at_start(mut self) -> Self {
        self.config_mut().insertion_point = Some(InsertionPoint::AtStart);
        self
    }
    fn before_commands(mut self) -> Self {
        self.config_mut().insertion_point = Some(InsertionPoint::BeforeCommands);
        self
    }
    fn at_end(mut self) -> Self {
        self.config_mut().insertion_point = Some(InsertionPoint::AtEnd);
        self
    }
}

impl<T, Params> ExclusiveConfig<Params, ExclusiveSystemFn> for T
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