use crate::{
    args::Args,
    commands::{
        BenchCheckCommand, CompileCheckCommand, CompileFailCommand, ExampleCheckCommand,
        IntegrationTestCheckCommand, TestCheckCommand,
    },
    Prepare, PreparedCommand,
};
use argh::FromArgs;

/// Alias for running the `compile-fail`, `bench-check`, `example-check`, `compile-check`, `test-check` and `test-integration-check` subcommands.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile")]
pub struct CompileCommand {}

impl Prepare for CompileCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let mut commands = vec![];
        commands.append(&mut CompileFailCommand::default().prepare(sh, args));
        commands.append(&mut BenchCheckCommand::default().prepare(sh, args));
        commands.append(&mut ExampleCheckCommand::default().prepare(sh, args));
        commands.append(&mut CompileCheckCommand::default().prepare(sh, args));
        commands.append(&mut TestCheckCommand::default().prepare(sh, args));
        commands.append(&mut IntegrationTestCheckCommand::default().prepare(sh, args));
        commands
    }
}
