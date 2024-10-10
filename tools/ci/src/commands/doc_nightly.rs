use crate::{
    commands::{DocCheckNightlyCommand, DocTestNightlyCommand},
    Flag, Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Alias for running the `doc-test-nightly` and `doc-check-nightly` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-nightly")]
pub struct DocNightlyCommand {}

impl Prepare for DocNightlyCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(&mut DocTestNightlyCommand::default().prepare(sh, flags));
        commands.append(&mut DocCheckNightlyCommand::default().prepare(sh, flags));
        commands
    }
}
