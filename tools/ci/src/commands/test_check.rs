use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all tests compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test-check")]
pub struct TestCheckCommand {}

impl Prepare for TestCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let quiet = flags
            .contains(Flag::QUIET)
            .then_some("--quiet")
            .unwrap_or_default();

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace --tests {quiet}"),
            "Please fix compiler examples for tests in output above.",
        )]
    }
}
