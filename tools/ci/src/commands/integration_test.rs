use crate::{commands::get_integration_tests, Flag, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all integration tests (except for doc tests).
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "integration-test")]
pub struct IntegrationTestCommand {}

impl Prepare for IntegrationTestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, flags: Flag) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = flags
            .contains(Flag::KEEP_GOING)
            .then_some("--no-fail-fast")
            .unwrap_or_default();

        get_integration_tests(sh)
            .into_iter()
            .map(|path| {
                PreparedCommand::new::<Self>(
                    cmd!(
                        sh,
                        "cargo test --manifest-path {path}/Cargo.toml --tests {no_fail_fast}"
                    ),
                    "Please fix failing integration tests in output above.",
                )
            })
            .collect()
    }
}
