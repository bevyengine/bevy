use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Checks that all tests compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test-check")]
pub struct TestCheckCommand {}

impl Prepare for TestCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo check --workspace --tests {jobs...}"),
            "Please fix compiler errors for tests in output above.",
        )]
    }
}
