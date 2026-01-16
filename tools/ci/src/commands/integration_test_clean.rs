use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

use super::get_integration_tests;

/// Cleans the build artifacts for all integration tests.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "integration-test-clean")]
pub struct IntegrationTestCleanCommand {}

impl Prepare for IntegrationTestCleanCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, _args: Args) -> Vec<PreparedCommand<'a>> {
        get_integration_tests(sh)
            .into_iter()
            .map(|path| {
                PreparedCommand::new::<Self>(
                    cmd!(sh, "cargo clean --manifest-path {path}/Cargo.toml"),
                    "Failed to clean integration test.",
                )
            })
            .collect()
    }
}
