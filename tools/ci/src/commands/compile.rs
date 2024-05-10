use crate::commands::{
    BenchCheckCommand, CompileCheckCommand, CompileFailCommand, ExampleCheckCommand,
    TestCheckCommand,
};
use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;

/// Alias for running the `compile-fail`, `bench-check`, `example-check`, `compile-check`, and `test-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile")]
pub struct CompileCommand {}

impl Prepare for CompileCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(&mut CompileFailCommand::default().prepare(sh, flags));
        commands.append(&mut BenchCheckCommand::default().prepare(sh, flags));
        commands.append(&mut ExampleCheckCommand::default().prepare(sh, flags));
        commands.append(&mut CompileCheckCommand::default().prepare(sh, flags));
        commands.append(&mut TestCheckCommand::default().prepare(sh, flags));
        commands
    }
}
