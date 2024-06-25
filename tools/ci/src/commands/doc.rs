use crate::commands::{DocCheckCommand, DocTestCommand};
use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;

/// Alias for running the `doc-test` and `doc-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc")]
pub struct DocCommand {
    #[argh(switch)]
    /// emit errors as json
    emit_json: bool,
}

impl Prepare for DocCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(
            &mut DocTestCommand::default()
                .with_json(self.emit_json)
                .prepare(sh, flags),
        );
        commands.append(
            &mut DocCheckCommand::default()
                .with_json(self.emit_json)
                .prepare(sh, flags),
        );
        commands
    }
}
