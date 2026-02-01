use crate::{args::Args, commands::get_integration_tests, Prepare, PreparedCommand};
use argh::FromArgs;
use xshell::cmd;

/// Runs all integration tests (except for doc tests).
#[derive(FromArgs, Default)]
#[argh(subcommand, name = "integration-test")]
pub struct IntegrationTestCommand {}

impl Prepare for IntegrationTestCommand {
    fn prepare<'a>(&self, sh: &'a xshell::Shell, args: Args) -> Vec<PreparedCommand<'a>> {
        let no_fail_fast = args.keep_going();
        let jobs = args.build_jobs();
        let test_threads = args.test_threads();
        let jobs_ref = jobs.as_ref();
        let test_threads_ref = test_threads.as_ref();

        get_integration_tests(sh)
            .into_iter()
            .map(|path| {
                PreparedCommand::new::<Self>(
                    cmd!(
                        sh,
                        "cargo test --manifest-path {path}/Cargo.toml --tests {no_fail_fast...} {jobs_ref...} -- {test_threads_ref...}"
                    ),
                    "Please fix failing integration tests in output above.",
                )
            })
            .collect()
    }
}
