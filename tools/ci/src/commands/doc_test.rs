use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all doc tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "doc-test")]
pub struct DocTestCommand {}

impl Prepare for DocTestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = flags
            .contains(Flag::KEEP_GOING)
            .then_some("--no-fail-fast")
            .unwrap_or_default();

        vec![PreparedCommand::new::<Self>(
            cmd!(sh, "cargo test --workspace --doc {no_fail_fast}"),
            "Please fix failing doc tests in output above.",
        )]
    }
}
