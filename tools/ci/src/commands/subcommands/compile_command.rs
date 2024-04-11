use crate::commands::subcommands::{
    BenchCheckCommand, CompileCheckCommand, CompileFailCommand, ExampleCheckCommand,
    TestCheckCommand,
};
use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;

/// Alias for running the `compile-fail`, `bench-check`, `example-check`, `compile-check`, and `test-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile")]
pub(crate) struct CompileCommand {
    /// runs all checks with the `--no-fail-fast` flag
    #[argh(switch, hidden_help)]
    keep_going: bool,
}

impl Prepare for CompileCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, mut flags: Flag) -> Vec<PreparedCommand<'a>> {
        if self.keep_going {
            flags |= Flag::KEEP_GOING;
        }

        let mut commands = vec![];
        commands.append(&mut CompileFailCommand::default().prepare(sh, flags));
        commands.append(&mut BenchCheckCommand::default().prepare(sh, flags));
        commands.append(&mut ExampleCheckCommand::default().prepare(sh, flags));
        commands.append(&mut CompileCheckCommand::default().prepare(sh, flags));
        commands.append(&mut TestCheckCommand::default().prepare(sh, flags));
        commands
    }
}
