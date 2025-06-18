use crate::{
    args::Args,
    commands::{ClippyCommand, FormatCommand},
    Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Alias for running the `format` and `clippy` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "lints")]
pub struct LintsCommand {}

impl Prepare for LintsCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(&mut FormatCommand::default().prepare(sh, args));
        commands.append(&mut ClippyCommand::default().prepare(sh, args));
        commands
    }
}
