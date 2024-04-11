use crate::commands::subcommands::{DocCheckCommand, DocTestCommand};
use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;

/// Alias for running the `doc-test` and `doc-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc")]
pub(crate) struct DocCommand {
    /// runs all checks with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for DocCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, mut flags: Flag) -> Vec<PreparedCommand<'a>> {
        if self.keep_going {
            flags |= Flag::KEEP_GOING;
        }

        let mut commands = vec![];
        commands.append(&mut DocTestCommand::default().prepare(sh, flags));
        commands.append(&mut DocCheckCommand::default().prepare(sh, flags));
        commands
    }
}
