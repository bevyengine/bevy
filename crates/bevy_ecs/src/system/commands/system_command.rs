use crate::system::{IntoSystem, SystemParam};

use super::{Command, IntoCommand};

#[doc(hidden)]
pub struct IsSystemCommand;

pub trait CommandParam: SystemParam {}

impl<Marker, T: IntoSystem<(), (), Marker>> IntoCommand<(IsSystemCommand, Marker)> for T
where
    T::System: Command,
{
    type Command = T::System;
    fn into_command(this: Self) -> Self::Command {
        IntoSystem::into_system(this)
    }
}
