use std::marker::PhantomData;

use crate::{
    prelude::World,
    system::{IntoSystem, SystemMeta, SystemParam, SystemParamFunction},
};

use super::{Command, IntoCommand};

pub trait CommandSystemParam: SystemParam {}

impl<Marker, T: IntoSystem<(), (), Marker>> IntoCommand<(IsSystemCommand, Marker)> for T
where
    T::System: Command,
{
    type Command = T::System;
    fn into_command(this: Self) -> Self::Command {
        IntoSystem::into_system(this)
    }
}

#[doc(hidden)]
pub struct IsSystemCommand;

pub struct SystemCommand<Param, Marker, F>
where
    Param: CommandSystemParam,
{
    func: F,
    system_meta: SystemMeta,
    marker: PhantomData<fn(Param) -> Marker>,
}

impl<Param, Marker, F> IntoCommand<(IsSystemCommand, Param, Marker)> for F
where
    Param: CommandSystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<(), (), Param, Marker> + Send + Sync + 'static,
{
    type Command = SystemCommand<Param, Marker, F>;
    fn into_command(func: Self) -> Self::Command {
        SystemCommand {
            func,
            system_meta: SystemMeta::new::<F>(),
            marker: PhantomData,
        }
    }
}

impl<Param, Marker, F> Command for SystemCommand<Param, Marker, F>
where
    Param: CommandSystemParam + 'static,
    Marker: 'static,
    F: SystemParamFunction<(), (), Param, Marker> + Send + Sync + 'static,
{
    fn write(mut self, world: &mut World) {
        let change_tick = world.change_tick();

        let mut param_state = Param::init_state(world, &mut self.system_meta);
        let params =
            // SAFETY: We have exclusive world access.
            unsafe { Param::get_param(&mut param_state, &self.system_meta, world, change_tick) };
        self.func.run((), params);
        Param::apply(&mut param_state, &self.system_meta, world);
    }
}
