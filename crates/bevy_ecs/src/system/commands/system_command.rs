use crate::system::IntoSystem;

use super::{Command, IntoCommand};

pub struct IsSystemCommand;

impl<Marker, T: IntoSystem<(), (), Marker>> IntoCommand<(IsSystemCommand, Marker)> for T
where
    T::System: Command,
{
    type Command = T::System;
    fn into_command(self) -> Self::Command {
        IntoSystem::into_system(self)
    }
}
