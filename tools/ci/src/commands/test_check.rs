use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all tests compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test-check")]
pub struct TestCheckCommand {}

impl Prepare for TestCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _flags: Flag) -> Vec<PreparedCommand<'a>> {
        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace --tests"),
            "Please fix compiler examples for tests in output above.",
        )]
    }
}
