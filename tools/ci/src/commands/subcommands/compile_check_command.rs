use crate::commands::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the project compiles.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "compile-check")]
pub(crate) struct CompileCheckCommand {}

impl Prepare for CompileCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace"),
            "Please fix compiler errors in output above.",
        )]
    }
}
