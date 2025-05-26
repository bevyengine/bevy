use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that the examples compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "example-check")]
pub struct ExampleCheckCommand {}

impl Prepare for ExampleCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace --examples"),
            "Please fix compiler errors for examples in output above.",
        )]
    }
}
