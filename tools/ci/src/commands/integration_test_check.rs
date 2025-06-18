use std::path::Path;

use crate::{args::Args, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

pub fn get_integration_tests(sh: &xshell::Shell) -> Vec<String> {
    let integration_test_paths = sh.read_dir(Path::new("./tests-integration")).unwrap();

    // Filter out non-directories
    integration_test_paths
        .into_iter()
        .filter(|path| path.is_dir())
        .map(|path| path.to_string_lossy().to_string())
        .collect()
}

/// Checks that all integration tests compile.
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "integration-test-check")]
pub struct IntegrationTestCheckCommand {}

impl Prepare for IntegrationTestCheckCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let jobs = args.build_jobs();
        let jobs_ref = jobs.as_ref();

        get_integration_tests(sh)
            .into_iter()
            .map(|path| {
                PreparedCommand::new::<Self>(
                    cmd!(
                        sh,
                        "cargo check --manifest-path {path}/Cargo.toml --tests {jobs_ref...}"
                    ),
                    "Please fix compiler errors for tests in output above.",
                )
            })
            .collect()
    }
}
