use crate::{
    args::Args,
    commands::{DocCheckCommand, DocTestCommand},
    Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Alias for running the `doc-test` and `doc-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc")]
pub struct DocCommand {}

impl Prepare for DocCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(&mut DocTestCommand::default().prepare(sh, args));
        commands.append(&mut DocCheckCommand::default().prepare(sh, args));
        commands
    }
}
