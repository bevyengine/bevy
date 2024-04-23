use crate::commands::subcommands::{DocCheckCommand, DocTestCommand};
use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;

/// Alias for running the `doc-test` and `doc-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc")]
pub(crate) struct DocCommand {}

impl Prepare for DocCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(&mut DocTestCommand::default().prepare(sh, flags));
        commands.append(&mut DocCheckCommand::default().prepare(sh, flags));
        commands
    }
}
