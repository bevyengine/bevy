use crate::{Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all tests (except for doc tests).
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "test")]
pub struct TestCommand {}

impl Prepare for TestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = flags
            .contains(Flag::KEEP_GOING)
            .then_some("--no-fail-fast")
            .unwrap_or_default();

        vec![
            PreparedCommand::new::<Self>(
                cmd!(
                    sh,
                    // `--benches` runs each benchmark once in order to verify that they behave
                    // correctly and do not panic.
                    "cargo test --workspace --lib --bins --benches {no_fail_fast}"
                ),
                "Please fix failing tests in output above.",
            ),
            PreparedCommand::new::<Self>(
                cmd!(
                    sh,
                    // `--benches` runs each benchmark once in order to verify that they behave
                    // correctly and do not panic.
                    "cargo test --tests {no_fail_fast} -- --skip check_mesh --skip check_standard_material"
                ),
                "Please fix failing tests in output above.",
            ),
        ]
    }
}
