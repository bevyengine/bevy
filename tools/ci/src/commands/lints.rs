use crate::commands::{ClippyCommand, FormatCommand};
use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;

/// Alias for running the `format` and `clippy` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "lints")]
pub struct LintsCommand {}

impl Prepare for LintsCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(&mut FormatCommand::default().prepare(sh, flags));
        commands.append(&mut ClippyCommand::default().prepare(sh, flags));
        commands
    }
}
